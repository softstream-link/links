use crate::{core::PollAccept, prelude::*};
use core::panic;
use log::{debug, info, log_enabled, warn, Level};
use mio::{Events, Poll, Token, Waker};
use slab::Slab;
use std::{
    io::Error,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    thread::Builder,
};

// setting up these macros to reuse code where borrow checker, iterating over self.events while modifying self.serviceable
macro_rules! register_recver_as_readable {
    ($self:ident, $recver:ident, $token:ident) => {
        // USING register method instead of source to enable overriding of register method when locking is required
        // $self.poll.registry().register(*$recver.source(), $token, mio::Interest::READABLE).expect("Failed to poll register recver");
        $recver.register($self.poll.registry(), $token, mio::Interest::READABLE).expect("Failed to poll register recver");
        if log_enabled!(Level::Debug) {
            debug!("registered recver: {} with token: {:?}", $recver.con_id(), $token);
        }
    };
}
macro_rules! register_acceptor_as_readable {
    ($self:ident, $acceptor:ident, $token:ident) => {
        // USING $acceptor.register method instead of $acceptor.source to enable overriding of register method when locking is required
        // $self.poll.registry().register(*$acceptor.source(), $token, mio::Interest::READABLE).expect("Failed to poll register acceptor");
        $acceptor.register($self.poll.registry(), $token, mio::Interest::READABLE).expect("Failed to poll register acceptor");
        if log_enabled!(Level::Debug) {
            debug!("registered acceptor: {} with token: {:?}", $acceptor.con_id(), $token);
        }
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
            Serviceable::Waker => panic!("Invalid API usage. Waker should not be manually registered as serviceable. It is auto registered when calling [PollHandler::into_spawned_handler]"),
        }
    };
}

enum Serviceable<R: PollRead, A: PollAccept<R>> {
    Acceptor(A),
    Recver(R),
    Waker,
}

/// A wrapper struct to that will use a designated thread to handle [TransmittingSvcAcceptor] or [TransmittingSvcAcceptorRef] events
/// and resulting respective [CltRecver] & [CltRecverRef] instances
pub struct PollHandler<R: PollRead, A: PollAccept<R>> {
    poll: Poll,
    serviceable: Slab<Serviceable<R, A>>,
    events: Events,
}
impl<R: PollRead, A: PollAccept<R>> PollHandler<R, A> {
    /// Create a new [PollHandler] with a given capacity of Events on a single poll call
    pub fn with_events_capacity(capacity: usize) -> Self {
        Self {
            poll: Poll::new().expect("Failed to create Poll"),
            serviceable: Slab::new(),
            events: Events::with_capacity(capacity),
        }
    }
    /// Add a [TransmittingSvcAcceptor] or [TransmittingSvcAcceptorRef] to the [PollHandler] to be polled for incoming connections. All resulting connections in the form
    /// of [CltRecver] will also be serviced by this [PollHandler] instance.
    pub fn add_acceptor(&mut self, acceptor: A) {
        self.add_serviceable(Serviceable::Acceptor(acceptor))
    }
    pub fn add_recver(&mut self, recver: R) {
        self.add_serviceable(Serviceable::Recver(recver))
    }
    /// Spawns a new thread with a given name that will continuously poll for events of [TransmittingSvcAcceptor] or [TransmittingSvcAcceptorRef] and resulting [CltRecver]s instances
    pub fn into_spawned_handler(mut self, name: &str) -> SpawnedPollHandler<R, A> {
        let waker = {
            let entry = self.serviceable.vacant_entry();
            let key = entry.key();
            let waker = Waker::new(self.poll.registry(), Token(key)).expect("Failed to create Waker");
            entry.insert(Serviceable::Waker);
            if log_enabled!(Level::Debug) {
                debug!("registering waker with token: {:?}", Token(key));
            }
            waker
        };
        // have to use synch_channel of just 1 so that if adding serviceable back to back the wake call on the poll is only issued after the first wake is processed
        // otherwise the poll will not wake up on back to back wake calls and serviceable will end up being stuck in the channel
        // let (tx_serviceable, rx_serviceable) = sync_channel::<Serviceable<R, A>>(1);
        let (tx_serviceable, rx_serviceable) = channel::<Serviceable<R, A>>();
        // let (tx_serviceable, rx_serviceable) = channel::<Serviceable<R, A>>();

        Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                // let rx_serviceable = rx_serviceable;
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

        loop {
            // keep going until all serviceable for the given poll events can't yield anymore
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
                            // USING recver.deregister method instead of recver.source to enable overriding of deregister method when locking is required
                            // self.poll.registry().deregister(*recver.source())?;
                            recver.deregister(self.poll.registry())?;
                            self.serviceable.remove(key);
                        }
                        Err(e) => {
                            if log_enabled!(Level::Warn) {
                                warn!("Error, service loop termination recver: {}, error: {}", recver, e);
                            }
                            // USING recver.deregister method instead of recver.source to enable overriding of deregister method when locking is required
                            // self.poll.registry().deregister(*recver.source())?;
                            recver.deregister(self.poll.registry())?;
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
                        Ok(AcceptStatus::Rejected) => {
                            had_yield = true;
                        }
                        Ok(AcceptStatus::WouldBlock) => continue,
                        Err(e) => {
                            if log_enabled!(Level::Warn) {
                                warn!("Error, service loop termination acceptor: {}, error: {}", acceptor, e);
                            }
                            // USING acceptor.deregister method instead of acceptor.source to enable overriding of deregister method when locking is required
                            // self.poll.registry().deregister(*acceptor.source())?;
                            acceptor.deregister(self.poll.registry())?;
                            self.serviceable.remove(key);
                        }
                    },
                    Some(Waker) => {
                        // logging here causes python to hang
                        match rx_serviceable.try_recv() {
                            Ok(serviceable) => {
                                register_serviceable_as_readable!(self, serviceable);
                                had_yield = true;
                            }
                            Err(TryRecvError::Empty) => {}
                            Err(e) => panic!("Could not receive Serviceable from rx_serviceable channel: {:?}. This is not a possible condition error: {}", rx_serviceable, e),
                        }
                    }
                    None => {} // possible when the serviceable is removed during error or terminate request but other serviceable still yielding
                }
            }
            // only return once every event in the for loop yields WouldBlock
            if !had_yield {
                return Ok(());
            }
        }
    }
}
impl<R: PollRead, A: PollAccept<R>> Default for PollHandler<R, A> {
    fn default() -> Self {
        Self::with_events_capacity(1024)
    }
}

impl PollAccept<Box<dyn PollRead>> for Box<dyn PollAccept<Box<dyn PollRead>>> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<Box<dyn PollRead>>, Error> {
        self.as_mut().poll_accept()
    }
}
impl PollAble for Box<dyn PollAccept<Box<dyn PollRead>>> {
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
}
impl ConnectionId for Box<dyn PollAccept<Box<dyn PollRead>>> {
    fn con_id(&self) -> &ConId {
        self.as_ref().con_id()
    }
}

impl PollRead for Box<dyn PollRead> {
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_readable_event()
    }
}
impl PollAble for Box<dyn PollRead> {
    fn register(&mut self, registry: &mio::Registry, token: Token, interests: mio::Interest) -> Result<(), Error> {
        self.as_mut().register(registry, token, interests)
    }
    fn deregister(&mut self, registry: &mio::Registry) -> Result<(), Error> {
        self.as_mut().deregister(registry)
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
}
impl ConnectionId for Box<dyn PollRead> {
    fn con_id(&self) -> &ConId {
        self.as_ref().con_id()
    }
}
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> From<CltRecver<P, C, MAX_MSG_SIZE>> for Box<dyn PollRead> {
    fn from(value: CltRecver<P, C, MAX_MSG_SIZE>) -> Self {
        Box::new(value)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> From<CltRecverRef<P, C, MAX_MSG_SIZE>> for Box<dyn PollRead> {
    fn from(value: CltRecverRef<P, C, MAX_MSG_SIZE>) -> Self {
        Box::new(value)
    }
}

/// A helper struct to add [PollAccept] and [PollRead] instances to a [PollHandler] from a different thread
/// to create an instance of this struct use [PollHandler::into_spawned_handler]
pub struct SpawnedPollHandler<R: PollRead, A: PollAccept<R>> {
    // tx_serviceable: SyncSender<Serviceable<R, A>>,
    tx_serviceable: Sender<Serviceable<R, A>>,
    waker: Waker,
}
impl<R: PollRead, A: PollAccept<R>> SpawnedPollHandler<R, A> {
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
/// A [PollHandler] that can handle any [PollAccept] and [PollRead] instances using dynamic dispatch at the cost of performance
pub type PollHandlerDynamic = PollHandler<Box<dyn PollRead>, Box<dyn PollAccept<Box<dyn PollRead>>>>;
pub type SpawnedPollHandlerDynamic = SpawnedPollHandler<Box<dyn PollRead>, Box<dyn PollAccept<Box<dyn PollRead>>>>;

/// A [PollHandler] that will only handle [PollAccept] and [PollRead] of same type
pub type PollHandlerStatic<P, C, const MAX_MSG_SIZE: usize> = PollHandler<CltRecver<P, C, MAX_MSG_SIZE>, TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE>>;
pub type SpawnedPollHandlerStatic<M, C, const MAX_MSG_SIZE: usize> = SpawnedPollHandler<CltRecver<M, C, MAX_MSG_SIZE>, TransmittingSvcAcceptor<M, C, MAX_MSG_SIZE>>;

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::{
        prelude::*,
        unittest::setup::protocol::{CltTestProtocolManual, SvcTestProtocolManual},
    };
    use links_core::unittest::setup::{
        self,
        messenger::TEST_MSG_FRAME_SIZE,
        model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg, SvcTestMsgDebug, UniTestMsg},
    };
    use log::info;
    use std::{num::NonZeroUsize, time::Instant};

    #[test]
    fn test_poller_static() {
        setup::log::configure_compact(log::LevelFilter::Info);

        let addr = setup::net::rand_avail_addr_port();
        let counter = CounterCallback::new_ref();
        let clbk = ChainCallback::new_ref(vec![LoggerCallback::new_ref(), counter.clone()]);
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), clbk, SvcTestProtocolManual::default(), Some("unittest/svc")).unwrap();

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        let mut poll_handler = PollHandlerStatic::<_, _, TEST_MSG_FRAME_SIZE>::default();
        poll_handler.add_acceptor(acceptor);

        let _ = poll_handler.into_spawned_handler("Static-Svc-Poll-Thread");

        let mut msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
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

        // test that second connection is denied due to svc having set the limit of 1 on max connections
        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("unittest/clt1"),
        )
        .unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_completed_none());
        // however after dropping clt a new connection can be established, drop will close the socket which svc will detect and allow a new connection
        drop(clt);
        let mut clt1: Clt<_, _, TEST_MSG_FRAME_SIZE> = Clt::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("unittest/clt"),
        )
        .unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_wouldblock());
    }

    #[test]
    fn test_poller_dynamic() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr1 = setup::net::rand_avail_addr_port();
        let addr2 = setup::net::rand_avail_addr_port();

        let store = CanonicalEntryStore::<UniTestMsg>::new_ref();

        let svc1 = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr1, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), SvcTestProtocolManual::default(), Some("unittest/svc1")).unwrap();
        let svc2 = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr2, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), SvcTestProtocolManual::default(), Some("unittest/svc2")).unwrap();

        let clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            StoreCallback::new_ref(store.clone()),
            CltTestProtocolManual::default(),
            Some("unittest/clt1"),
        )
        .unwrap();
        let clt2 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr2,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            StoreCallback::new_ref(store.clone()),
            CltTestProtocolManual::default(),
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
        poll_adder.add_recver(Box::new(clt2_recver));

        clt1.send_busywait(&mut CltTestMsgDebug::new(b"Hello From Clt1").into()).unwrap();
        clt2.send_busywait(&mut CltTestMsgDebug::new(b"Hello From Clt2").into()).unwrap();
        svc1.send_busywait(&mut SvcTestMsgDebug::new(b"Hello From Svc1").into()).unwrap();
        svc2.send_busywait(&mut SvcTestMsgDebug::new(b"Hello From Svc2").into()).unwrap();

        let found = store.find_recv("unittest/svc1", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Clt(CltTestMsg::Dbg(msg)) if msg == CltTestMsgDebug::new(b"Hello From Clt1")));

        let found = store.find_recv("unittest/svc2", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Clt(CltTestMsg::Dbg(msg)) if msg == CltTestMsgDebug::new(b"Hello From Clt2")));

        let found = store.find_recv("unittest/clt1", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Svc(SvcTestMsg::Dbg(msg)) if msg == SvcTestMsgDebug::new(b"Hello From Svc1")));

        let found = store.find_recv("unittest/clt2", |_x| true, setup::net::optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Svc(SvcTestMsg::Dbg(msg)) if msg == SvcTestMsgDebug::new(b"Hello From Svc2")));

        info!("store: {}", store);
    }
}
