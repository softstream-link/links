use std::{
    io::{self, Error},
    thread::{Builder, JoinHandle},
};

use log::{info, log_enabled, Level};
use mio::{Events, Poll, Token};
use slab::Slab;

use crate::{core::PollAccept, prelude::*};
pub enum Serviceable<R: PollRecv, A: PollAccept<R>> {
    Acceptor(A),
    Recver(R),
}

/// A wrapper struct to that will use a designated thread to handle all of its [PoolCltAcceptor]s events and resulting [CltRecver]s
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
    /// Add a [PoolCltAcceptor] to the [PollHandler] to be polled for incoming connections. All resulting connections in the form
    /// of [CltRecver] will also be serviced by this [PollHandler] instance.
    pub fn add(&mut self, acceptor: A) -> io::Result<()> {
        self.add_serviceable(Serviceable::Acceptor(acceptor))
    }
    /// Spawns a new thread with a given name that will continuously poll for events on all of its [PoolCltAcceptor]s and resulting [CltRecver]s instances
    pub fn spawn(mut self, name: &str) -> JoinHandle<()> {
        Builder::new()
            .name(name.to_owned())
            .spawn(move || loop {
                match self.service() {
                    Ok(_) => {}
                    Err(e) => {
                        panic!("Error, service loop termination: {}", e);
                    }
                }
            })
            .unwrap_or_else(|_| panic!("Failed to start a poll thread name: '{}'", name))
    }

    fn add_serviceable(&mut self, recver: Serviceable<R, A>) -> io::Result<()> {
        let key = self.serviceable.insert(recver);
        match self.serviceable[key] {
            Serviceable::Recver(ref mut recver) => {
                self.poll
                    .registry()
                    .register(*recver.source(), Token(key), mio::Interest::READABLE)
            }
            Serviceable::Acceptor(ref mut acceptor) => self.poll.registry().register(
                *acceptor.source(),
                Token(key),
                mio::Interest::READABLE,
            ),
        }
    }

    pub fn service(&mut self) -> Result<(), Error> {
        use PollEventStatus::*;
        use Serviceable::*;
        self.poll.poll(&mut self.events, None)?;

        let mut at_least_one_completed = true;
        while at_least_one_completed {
            at_least_one_completed = false;

            for event in self.events.iter() {
                let token = event.token().into();
                match self.serviceable.get_mut(token) {
                    // handle readable event
                    Some(Recver(recver)) => match recver.on_readable_event() {
                        Ok(Completed) => {
                            at_least_one_completed = true;
                            continue;
                        }
                        Ok(WouldBlock) => continue,
                        Ok(Terminate) => {
                            if log_enabled!(Level::Info) {
                                info!("Clean, service loop termination recver: {}", recver);
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(token);
                        }
                        Err(e) => {
                            if log_enabled!(Level::Info) {
                                info!(
                                    "Error, service loop termination recver: {}, error: {}",
                                    recver, e
                                );
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(token);
                        }
                    },
                    // accept new connection and register it for READABLE events
                    Some(Acceptor(acceptor)) => match acceptor.poll_accept() {
                        Ok(AcceptStatus::Accepted(recver)) => {
                            let key = self.serviceable.insert(Recver(recver));
                            if let Recver(ref mut recver) = self.serviceable[key] {
                                self.poll.registry().register(
                                    *recver.source(),
                                    Token(key),
                                    mio::Interest::READABLE,
                                )?;
                            }
                            at_least_one_completed = true;
                        }
                        Ok(AcceptStatus::WouldBlock) => {}
                        Err(e) => {
                            if log_enabled!(Level::Info) {
                                info!(
                                    "Error, service loop termination acceptor: {}, error: {}",
                                    acceptor, e
                                );
                            }
                            self.poll.registry().deregister(*acceptor.source())?;
                            self.serviceable.remove(token);
                        }
                    },
                    None => {} // possible when the serviceable is removed with error or terminate from the service loop but other serviceable are still responding in this iteration
                }
            }
        }
        Ok(())
    }
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
}
impl PollRecv for Box<dyn PollAccept<Box<dyn PollRecv>>> {
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_readable_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
}

impl PollRecv for Box<dyn PollRecv> {
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_readable_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
}

/// A [PollHandler] that can handle any [PollAccept] and [PollRecv] instances using dynamic dispatch at the cost of performance
pub type PollHandlerDynamic =
    PollHandler<Box<dyn PollRecv>, Box<dyn PollAccept<Box<dyn PollRecv>>>>;

/// A [PollHandler] that will only handle [PollAccept] and [PollRecv] of same type
pub type PollHandlerStatic<M, C, const MAX_MSG_SIZE: usize> =
    PollHandler<CltRecver<M, C, MAX_MSG_SIZE>, PoolCltAcceptor<M, C, MAX_MSG_SIZE>>;

#[cfg(test)]
mod test {
    use std::{num::NonZeroUsize, thread::sleep, time::Duration};

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
        setup::log::configure_level(log::LevelFilter::Info);

        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<SvcTestMessenger>::new_ref(),
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc"),
        )
        .unwrap();

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::<CltTestMessenger>::new_ref(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        let mut poll_handler = PollHandlerStatic::<_, _, TEST_MSG_FRAME_SIZE>::default();
        poll_handler.add(acceptor).unwrap();

        poll_handler.spawn("Svc-Poll");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..2 {
            clt.send_busywait(&mut msg).unwrap();
        }

        sleep(Duration::from_millis(200));
    }

    #[test]
    fn test_poller_dynamic() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr1 = setup::net::rand_avail_addr_port();
        let addr2 = setup::net::rand_avail_addr_port();

        let store = CanonicalEntryStore::<TestMsg>::new_ref();

        let svc1 = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(
            addr1,
            StoreCallback::new_ref(store.clone()),
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc1"),
        )
        .unwrap();

        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::<CltTestMessenger>::new_ref(),
            Some("unittest/clt1"),
        )
        .unwrap();

        let svc2 = Svc::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(
            addr2,
            StoreCallback::new_ref(store.clone()),
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc2"),
        )
        .unwrap();

        let mut clt2 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr2,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::<SvcTestMessenger>::new_ref(),
            Some("unittest/clt2"),
        )
        .unwrap();

        let (acceptor1, _, _sender_pool_1) = svc1.into_split();
        let (acceptor2, _, _sender_pool_2) = svc2.into_split();

        let mut poll_handler = PollHandlerDynamic::default();
        poll_handler.add(acceptor1.into()).unwrap();
        poll_handler.add(acceptor2.into()).unwrap();

        poll_handler.spawn("Svc-Poll");

        let mut msg1 = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Clt Messenger"));
        let mut msg2 = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Svc Messenger "));

        clt1.send_busywait(&mut msg1).unwrap();
        clt2.send_busywait(&mut msg2).unwrap();

        let found = store
            .find_recv(
                "unittest/svc1",
                |_x| true,
                setup::net::optional_find_timeout(),
            )
            .unwrap();
        info!("found: {:?}", found);

        let found = store
            .find_recv(
                "unittest/svc2",
                |_x| true,
                setup::net::optional_find_timeout(),
            )
            .unwrap();
        info!("found: {:?}", found);

        info!("store: {}", store);
    }
}
