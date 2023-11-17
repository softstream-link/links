use core::panic;
use log::{debug, info, log_enabled, warn, Level};
use mio::{Events, Poll, Token, Waker};
use slab::Slab;
use std::{
    io::Error,
    sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError},
    thread::Builder,
};

use crate::{core::PollAccept, prelude::*};

// setting up these macros to reuse code where borrow checker, iterating over self.events while modifying self.serviceable
macro_rules! register_recver_as_readable {
    ($self:ident, $recver:ident, $token:ident) => {
        if log_enabled!(Level::Debug) {
            debug!("registering recver: {} with token: {:?}", $recver.con_id(), $token);
        }
        $self
            .poll
            .registry()
            .register(*$recver.source(), $token, mio::Interest::READABLE)
            .expect("Failed to poll register recver")
    };
}
macro_rules! register_acceptor_as_readable {
    ($self:ident, $acceptor:ident, $token:ident) => {
        if log_enabled!(Level::Debug) {
            debug!("registering acceptor: {} with token: {:?}", PollAccept::con_id($acceptor), $token);
        }
        $self
            .poll
            .registry()
            .register(*$acceptor.source(), $token, mio::Interest::READABLE)
            .expect("Failed to poll register acceptor")
    };
}
macro_rules! register_serviceable_as_readable {
    ($self:ident, $serviceable:ident) => {
        let token = Token($self.serviceable.insert($serviceable));
        match $self.serviceable[token.into()] {
            Serviceable::Recver(ref mut recver) => {
                register_recver_as_readable!($self, recver, token);
            }
            Serviceable::Acceptor(ref mut acceptor) => {
                register_acceptor_as_readable!($self, acceptor, token);
            }
            Serviceable::Waker => panic!("Waker should not be added to the poll only when spawning a new thread"),
        }
    };
}

enum Serviceable<R: PollRecv, A: PollAccept<R>> {
    Acceptor(A),
    Recver(R),
    Waker,
}

/// A wrapper struct to that will use a designated thread to handle all of its [SvcPoolAcceptor]s events and resulting [CltRecver]s
pub struct PollHandler<R: PollRecv, A: PollAccept<R>> {
    poll: Poll,
    serviceable: Slab<Serviceable<R, A>>,
    events: Events,
}
impl<R: PollRecv, A: PollAccept<R>> PollHandler<R, A> {
    /// Create a new [PollHandler] with a given capacity of Events on a single poll call
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            poll: Poll::new().expect("Failed to create Poll"),
            serviceable: Slab::new(),
            events: Events::with_capacity(capacity),
        }
    }
    /// Add a [SvcPoolAcceptor] to the [PollHandler] to be polled for incoming connections. All resulting connections in the form
    /// of [CltRecver] will also be serviced by this [PollHandler] instance.
    pub fn add_acceptor(&mut self, acceptor: A) {
        self.add_serviceable(Serviceable::Acceptor(acceptor))
    }
    pub fn add_recver(&mut self, recver: R) {
        self.add_serviceable(Serviceable::Recver(recver))
    }
    /// Spawns a new thread with a given name that will continuously poll for events on all of its [SvcPoolAcceptor]s and resulting [CltRecver]s instances
    pub fn into_spawned_handler(mut self, name: &str) -> SpawnedPollHandler<R, A> {
        let waker = {
            let entry = self.serviceable.vacant_entry();
            let key = entry.key();
            let waker = Waker::new(self.poll.registry(), Token(key)).expect("Failed to create Waker");
            entry.insert(Serviceable::Waker);
            waker
        };
        // have to use synch_channel of just 1 so that if adding serviceable back to back the wake call on the poll is only issued after the first wake is processed
        // otherwise the poll will not wake up on back to back wake calls and serviceable will end up being stuck in the channel
        let (tx_serviceable, rx_serviceable) = sync_channel::<Serviceable<R, A>>(1);
        // let (tx_serviceable, rx_serviceable) = channel::<Serviceable<R, A>>();

        Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let rx_serviceable = rx_serviceable;
                loop {
                    match self.service(&rx_serviceable) {
                        Ok(()) => {}
                        Err(e) => {
                            panic!("Error, service loop termination: {}", e);
                        }
                    }
                }
            })
            .unwrap_or_else(|_| panic!("Failed to start a poll thread name: '{}'", name));
        SpawnedPollHandler { tx_serviceable, waker }
    }

    fn add_serviceable(&mut self, serviceable: Serviceable<R, A>) {
        register_serviceable_as_readable!(self, serviceable);
    }

    fn service(&mut self, rx_serviceable: &Receiver<Serviceable<R, A>>) -> Result<(), Error> {
        use PollEventStatus::*;
        use Serviceable::*;
        self.poll.poll(&mut self.events, None)?;

        // keep going until all serviceable for the given poll events can't yield anymore
        loop {
            let mut had_yield = false;
            for event in &self.events {
                let key = event.token().into();
                let serviceable = self.serviceable.get_mut(key);
                match serviceable {
                    Some(Recver(recver)) => match recver.on_readable_event() {
                        Ok(Completed) => {
                            had_yield = true;
                            continue;
                        }
                        Ok(WouldBlock) => continue,
                        Ok(Terminate) => {
                            if log_enabled!(Level::Info) {
                                info!("Clean, service loop termination recver: {}", recver);
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(key);
                        }
                        Err(e) => {
                            if log_enabled!(Level::Warn) {
                                warn!("Error, service loop termination recver: {}, error: {}", recver, e);
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(key);
                        }
                    },
                    Some(Acceptor(acceptor)) => match acceptor.poll_accept() {
                        Ok(AcceptStatus::Accepted(recver)) => {
                            let token = Token(self.serviceable.insert(Recver(recver)));
                            if let Recver(ref mut recver) = self.serviceable[token.into()] {
                                register_recver_as_readable!(self, recver, token);
                            }
                            had_yield = true;
                        }
                        Ok(AcceptStatus::WouldBlock) => continue,
                        Err(e) => {
                            if log_enabled!(Level::Warn) {
                                warn!("Error, service loop termination acceptor: {}, error: {}", acceptor, e);
                            }
                            self.poll.registry().deregister(*acceptor.source())?;
                            self.serviceable.remove(key);
                        }
                    },
                    Some(Waker) => match rx_serviceable.try_recv() {
                        Ok(serviceable) => {
                            register_serviceable_as_readable!(self, serviceable);
                        }
                        Err(e) if e == TryRecvError::Empty => {}
                        Err(e) => panic!(
                            "Could not receive Serviceable from rx_serviceable channel: {:?}. This is not a possible condition error: {}",
                            rx_serviceable, e
                        ),
                    },
                    None => {} // possible when the serviceable is removed during error or terminate request but other serviceable still yielding
                }
            }
            if !had_yield {
                return Ok(());
            }
        }
    }
    // pub fn servic
}
impl<R: PollRecv, A: PollAccept<R>> Default for PollHandler<R, A> {
    fn default() -> Self {
        Self::with_capacity(1024)
    }
}

impl PollAccept<Box<dyn PollRecv>> for Box<dyn PollAccept<Box<dyn PollRecv>>> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<Box<dyn PollRecv>>, Error> {
        self.as_mut().poll_accept()
    }
    fn con_id(&self) -> &ConId {
        PollAccept::con_id(self.as_ref())
    }
}
impl PollRecv for Box<dyn PollAccept<Box<dyn PollRecv>>> {
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_readable_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
    fn con_id(&self) -> &ConId {
        PollRecv::con_id(self.as_ref())
    }
}

impl PollRecv for Box<dyn PollRecv> {
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_readable_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
    fn con_id(&self) -> &ConId {
        self.as_ref().con_id()
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> From<CltRecver<M, C, MAX_MSG_SIZE>> for Box<dyn PollRecv> {
    fn from(value: CltRecver<M, C, MAX_MSG_SIZE>) -> Self {
        Box::new(value)
    }
}

/// A helper struct to add [PollAccept] and [PollRecv] instances to a [PollHandler] from a different thread
/// to create an instance of this struct use [PollHandler::into_spawned_handler]
pub struct SpawnedPollHandler<R: PollRecv, A: PollAccept<R>> {
    tx_serviceable: SyncSender<Serviceable<R, A>>,
    waker: Waker,
}
impl<R: PollRecv, A: PollAccept<R>> SpawnedPollHandler<R, A> {
    pub fn add_acceptor(&self, acceptor: A) {
        self.tx_serviceable.send(Serviceable::Acceptor(acceptor)).expect("Failed to send acceptor to PollHandler");
        self.waker.wake().expect("Failed to wake PollHandler after sending acceptor");
    }
    pub fn add_recver(&self, recver: R) {
        self.tx_serviceable.send(Serviceable::Recver(recver)).expect("Failed to send recver to PollHandler");
        self.waker.wake().expect("Failed to wake PollHandler after sending recver");
    }
    pub fn wake(&self) {
        self.waker.wake().expect("Failed to wake PollHandler");
    }
}
/// A [PollHandler] that can handle any [PollAccept] and [PollRecv] instances using dynamic dispatch at the cost of performance
pub type PollHandlerDynamic = PollHandler<Box<dyn PollRecv>, Box<dyn PollAccept<Box<dyn PollRecv>>>>;

/// A [PollHandler] that will only handle [PollAccept] and [PollRecv] of same type
pub type PollHandlerStatic<M, C, const MAX_MSG_SIZE: usize> = PollHandler<CltRecver<M, C, MAX_MSG_SIZE>, SvcPoolAcceptor<M, C, MAX_MSG_SIZE>>;

#[cfg(test)]
mod test {
    use std::{num::NonZeroUsize, time::Instant};

    use crate::prelude::*;
    use links_core::unittest::setup::{
        self,
        messenger::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        messenger_old::CltTestMessenger,
        model::{TestCltMsg, TestCltMsgDebug, TestMsg, TestSvcMsg, TestSvcMsgDebug},
    };
    use log::info;

    #[test]
    fn test_poller_static() {
        setup::log::configure_level(log::LevelFilter::Debug);

        let addr = setup::net::rand_avail_addr_port();
        let counter = CounterCallback::new_ref();
        let clbk = ChainCallback::new_ref(vec![LoggerCallback::new_ref(), counter.clone()]);

        let svc: Svc<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE> = Svc::bind(addr, clbk, NonZeroUsize::new(1).unwrap(), Some("unittest/svc")).unwrap();

        let mut clt: Clt<CltTestMessenger, _, TEST_MSG_FRAME_SIZE> = Clt::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        let mut poll_handler = PollHandlerStatic::<_, _, TEST_MSG_FRAME_SIZE>::default();
        poll_handler.add_acceptor(acceptor);

        let _ = poll_handler.into_spawned_handler("Static-Svc-Poll-Thread");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let write_count = 10;
        for _ in 0..write_count {
            clt.send_busywait(&mut msg).unwrap();
        }

        let start = Instant::now();
        while start.elapsed() < setup::net::optional_find_timeout().unwrap() {
            if counter.recv_count() == write_count {
                break;
            }
        }
        assert_eq!(counter.recv_count(), write_count);
    }

    #[test]
    fn test_poller_dynamic() {
        setup::log::configure_level(log::LevelFilter::Debug);

        let addr1 = setup::net::rand_avail_addr_port();
        let addr2 = setup::net::rand_avail_addr_port();

        let store = CanonicalEntryStore::<TestMsg>::new_ref();

        let svc1 = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(addr1, StoreCallback::new_ref(store.clone()), NonZeroUsize::new(1).unwrap(), Some("unittest/svc1")).unwrap();
        let svc2 = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(addr2, StoreCallback::new_ref(store.clone()), NonZeroUsize::new(1).unwrap(), Some("unittest/svc2")).unwrap();

        let clt1 = Clt::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            StoreCallback::new_ref(store.clone()),
            Some("unittest/clt1"),
        )
        .unwrap();
        let clt2 = Clt::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::connect(
            addr2,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            StoreCallback::new_ref(store.clone()),
            Some("unittest/clt2"),
        )
        .unwrap();

        let (acceptor1, _, mut svc1) = svc1.into_split();
        let (acceptor2, _, mut svc2) = svc2.into_split();
        let (clt1_recver, mut clt1) = clt1.into_split();
        let (clt2_recver, mut clt2) = clt2.into_split();

        let mut poll_handler = PollHandlerDynamic::default();
        // try adding before spawning
        poll_handler.add_acceptor(acceptor1.into());
        poll_handler.add_acceptor(acceptor2.into());

        let poll_adder = poll_handler.into_spawned_handler("Dynamic-Svc/Clt-Poll-Thread");
        // try adding after spawning
        poll_adder.add_recver(Box::new(clt1_recver));
        poll_adder.add_recver(clt2_recver.into());

        clt1.send_busywait(&mut TestCltMsgDebug::new(b"Hello From Clt1").into()).unwrap();
        clt2.send_busywait(&mut TestCltMsgDebug::new(b"Hello From Clt2").into()).unwrap();
        svc1.send_busywait(&mut TestSvcMsgDebug::new(b"Hello From Svc1").into()).unwrap();
        svc2.send_busywait(&mut TestSvcMsgDebug::new(b"Hello From Svc2").into()).unwrap();

        let found = store.find_recv("unittest/svc1", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), TestMsg::Clt(TestCltMsg::Dbg(msg)) if msg == TestCltMsgDebug::new(b"Hello From Clt1")));

        let found = store.find_recv("unittest/svc2", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), TestMsg::Clt(TestCltMsg::Dbg(msg)) if msg == TestCltMsgDebug::new(b"Hello From Clt2")));

        let found = store.find_recv("unittest/clt1", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), TestMsg::Svc(TestSvcMsg::Dbg(msg)) if msg == TestSvcMsgDebug::new(b"Hello From Svc1")));

        let found = store.find_recv("unittest/clt2", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), TestMsg::Svc(TestSvcMsg::Dbg(msg)) if msg == TestSvcMsgDebug::new(b"Hello From Svc2")));

        info!("store: {}", store);
    }
}
