use crate::{core::PollAccept, prelude::*};
use core::panic;
use log::{debug, info, log_enabled, warn, Level};
use mio::{Events, Poll, Token, Waker};
use slab::Slab;
use std::{
    fmt::Display,
    io::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender, TryRecvError},
    },
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
            Serviceable::Waker(_) => panic!("Invalid API usage. Waker should not be manually registered as serviceable. It is auto registered when calling [PollHandler::into_spawned_handler]"),
        }
    };
}

macro_rules! deregister_and_drop_some_serviceable {
    ($self:ident, $con_id:expr) => {
        // for mut serviceable in $self.serviceable.drain() {
        //     match serviceable {
        //         Recver(ref mut recver) => {
        //             // USING recver.deregister method instead of recver.source to enable overriding of deregister method when locking is required
        //             // self.poll.registry().deregister(*recver.source())?;
        //             recver.deregister($self.poll.registry()).expect(format!("Failed to deregister recver: {}", recver).as_str());
        //         }
        //         Acceptor(ref mut acceptor) => {
        //             // USING acceptor.deregister method instead of acceptor.source to enable overriding of deregister method when locking is required
        //             // self.poll.registry().deregister(*acceptor.source())?;
        //             acceptor.deregister($self.poll.registry()).expect(format!("Failed to deregister acceptor: {}", acceptor).as_str());
        //         }
        //         Waker(_) => {}
        //     }
        // }
        $self.serviceable.retain(|_k, s| match s {
            Recver(ref mut recver) => {
                if $con_id.is_none() || ($con_id.is_some() && $con_id.unwrap().from_same_lineage(recver.con_id())) {
                    // USING recver.deregister method instead of recver.source to enable overriding of deregister method when locking is required
                    // self.poll.registry().deregister(*recver.source())?;
                    recver.deregister($self.poll.registry()).unwrap();
                    false // don't retain
                } else {
                    true
                }
            }
            Acceptor(ref mut acceptor) => {
                if $con_id.is_none() || ($con_id.is_some() && $con_id.unwrap().from_same_lineage(acceptor.con_id())) {
                    // USING acceptor.deregister method instead of acceptor.source to enable overriding of deregister method when locking is required
                    // self.poll.registry().deregister(*acceptor.source())?;
                    acceptor.deregister($self.poll.registry()).unwrap();
                    false // don't retain
                } else {
                    true
                }
            }
            Waker(_) => {
                if $con_id.is_none() {
                    // if is_some it means we only shutting down a specific connection id and not terminating
                    false // don't retain
                } else {
                    true
                }
            }
        });
    };
}

enum ServiceStatus {
    Continue,
    Terminate,
}
enum Serviceable<R: PollRead, A: PollAccept<R>> {
    Acceptor(A),
    Recver(R),
    Waker(Option<ConId>),
}
impl<R: PollRead, A: PollAccept<R>> Display for Serviceable<R, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = asserted_short_name!("Serviceable", Self);
        match self {
            Serviceable::Acceptor(acceptor) => write!(f, "{}::Acceptor({})", name, acceptor.con_id()),
            Serviceable::Recver(recver) => write!(f, "{}::Recver({})", name, recver.con_id()),
            Serviceable::Waker(opt) => write!(f, "{}::Waker({})", name, {
                if let Some(ref con_id) = opt {
                    format!("{}", con_id)
                } else {
                    "None".to_owned()
                }
            }),
        }
    }
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
            entry.insert(Serviceable::Waker(None));
            if log_enabled!(Level::Debug) {
                debug!("{}::into_spawned_handler registering waker with token: {:?}", asserted_short_name!("PollHandler", Self), Token(key));
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
            .spawn(move || loop {
                match self.service(&rx_serviceable) {
                    Ok(ServiceStatus::Continue) => {}
                    Ok(ServiceStatus::Terminate) => break,
                    Err(e) => {
                        panic!("Error, service loop termination: {}", e);
                    }
                }
            })
            .unwrap_or_else(|_| panic!("Failed to start a poll thread name: '{}'", name));
        SpawnedPollHandler {
            tx_serviceable,
            waker,
            total_shutdown: AtomicBool::new(false),
        }
    }

    fn add_serviceable(&mut self, serviceable: Serviceable<R, A>) {
        register_serviceable_as_readable!(self, serviceable);
    }

    fn service(&mut self, rx_serviceable: &Receiver<Serviceable<R, A>>) -> Result<ServiceStatus, Error> {
        use PollEventStatus::*;
        use Serviceable::*;
        self.poll.poll(&mut self.events, None)?;

        loop {
            // keep going until all serviceable for the given poll events can't yield anymore
            let mut had_yield = false;
            let mut iteration = 0;
            for event in &self.events {
                iteration += 1;
                let key = event.token().into();
                let opt = self.serviceable.get_mut(key);
                // below if else ==> None  // possible when the serviceable is removed during error or terminate request but other serviceable still yielding
                if let Some(serviceable) = opt {
                    if log_enabled!(Level::Debug) {
                        debug!("Iteration #{}, Servicing {} with token: {:?} 1 of #{}", iteration, serviceable, Token(key), self.events.iter().count());
                    }

                    match serviceable {
                        // FROM self.serviceable.get_mut(key)
                        Recver(recver) => match recver.on_readable_event() {
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
                        // FROM self.serviceable.get_mut(key)
                        Acceptor(acceptor) => match acceptor.poll_accept() {
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
                        // FROM self.serviceable.get_mut(key)
                        Waker(None) => match rx_serviceable.try_recv() {
                            Ok(serviceable) => {
                                if log_enabled!(Level::Debug) {
                                    debug!("Waker received new serviceable: {}", serviceable);
                                }
                                match serviceable {
                                    Waker(None) => {
                                        if log_enabled!(Level::Warn) {
                                            warn!(
                                                "{} Waker received Waker, this will result in any active Receivers & Acceptors to be deregistered and dropped",
                                                asserted_short_name!("PollHandler", Self)
                                            );
                                        }
                                        deregister_and_drop_some_serviceable!(self, None::<ConId>);
                                        return Ok(ServiceStatus::Terminate);
                                    }
                                    Waker(Some(con_id)) => {
                                        if log_enabled!(Level::Info) {
                                            info!(
                                                "{} Waker received cond_id: {}, this will result in any active Receivers & Acceptors that share lineage to be deregistered and dropped",
                                                asserted_short_name!("PollHandler", Self),
                                                con_id
                                            );
                                        }
                                        deregister_and_drop_some_serviceable!(self, Some(con_id.clone()));
                                    }
                                    Acceptor(_) | Recver(_) => {
                                        register_serviceable_as_readable!(self, serviceable);
                                        had_yield = true;
                                    }
                                }
                            }
                            Err(TryRecvError::Empty) => {}
                            Err(TryRecvError::Disconnected) => {
                                if log_enabled!(Level::Warn) {
                                    warn!(
                                        "{} rx_serviceable channel is `Disconnected`, this will result in any active Receivers & Acceptors to be deregistered and dropped",
                                        asserted_short_name!("PollHandler", Self)
                                    );
                                }
                                deregister_and_drop_some_serviceable!(self, None::<ConId>);
                                return Ok(ServiceStatus::Terminate);
                            }
                        },
                        // self.serviceable.get_mut(key) can never yield Wake(Some(_)) because only Waker(None) is added to the self.serviceable and only on [PollHandler::into_spawned_handler]
                        // Waker(Some(_)) however can be sent via rx_serviceable which is why when Waker(None) branch must check for both Waker(None) and Waker(Some(_)) variants
                        Waker(Some(_)) => {
                            panic!("Invalid API usage. Waker can only be registered once and as Waker(None)")
                        }
                    }
                }
            }
            // only return once every event in the for loop yields WouldBlock
            if !had_yield {
                return Ok(ServiceStatus::Continue);
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
    tx_serviceable: Sender<Serviceable<R, A>>,
    waker: Waker,
    total_shutdown: AtomicBool,
}
impl<R: PollRead, A: PollAccept<R>> SpawnedPollHandler<R, A> {
    pub fn add_acceptor(&self, acceptor: A) {
        self.total_shutdown_check();
        if log_enabled!(Level::Debug) {
            debug!("{}::add_acceptor sending acceptor: {} to PollHandler and called waker", asserted_short_name!("SpawnedPollHandler", Self), acceptor);
        }
        self.tx_serviceable.send(Serviceable::Acceptor(acceptor)).expect("Failed to send acceptor to PollHandler");
        self.waker.wake().expect("Failed to wake PollHandler after sending acceptor");
    }
    pub fn add_recver(&self, recver: R) {
        self.total_shutdown_check();
        if log_enabled!(Level::Debug) {
            debug!("{}::add_recver sending recver: {} to PollHandler and called waker", asserted_short_name!("SpawnedPollHandler", Self), recver);
        }
        self.tx_serviceable.send(Serviceable::Recver(recver)).expect("Failed to send recver to PollHandler");
        self.waker.wake().expect("Failed to wake PollHandler after sending recver");
    }
    pub fn shutdown(&self, con_id: Option<ConId>) {
        if self.total_shutdown.load(Ordering::Acquire) {
            return;
        } else if con_id.is_none() {
            self.total_shutdown.store(true, Ordering::Release);
        }
        self.tx_serviceable.send(Serviceable::Waker(con_id.clone())).expect("Failed to send waker/terminate to PollHandler");
        self.waker.wake().expect("Failed to wake PollHandler after sending waker/terminate");
        if log_enabled!(Level::Debug) {
            debug!("{}::shutdown sent Waker({con_id:?}) to PollHandler and called waker", asserted_short_name!("SpawnedPollHandler", Self));
        }
    }
    pub fn wake(&self) {
        self.total_shutdown_check();
        self.waker.wake().expect("Failed to wake PollHandler");
        if log_enabled!(Level::Debug) {
            debug!("{}::wake to PollHandler", asserted_short_name!("SpawnedPollHandler", Self));
        }
    }
    #[track_caller]
    fn total_shutdown_check(&self) {
        if self.total_shutdown.load(Ordering::Relaxed) {
            panic!(
                "Invalid API usage. Trying to use {} after {}::shutdown(None) has been issued.",
                asserted_short_name!("SpawnedPollHandler", Self),
                asserted_short_name!("SpawnedPollHandler", Self)
            );
        }
    }
}
impl<R: PollRead, A: PollAccept<R>> Drop for SpawnedPollHandler<R, A> {
    fn drop(&mut self) {
        self.shutdown(None);
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
        unittest::setup::{
            connection::{CltTest, SvcTest},
            protocol::{CltTestProtocolManual, SvcTestProtocolManual},
        },
    };
    use links_core::unittest::setup::{
        self,
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
        let svc = SvcTest::bind(addr, NonZeroUsize::new(1).unwrap(), clbk, SvcTestProtocolManual::default(), Some("unittest/svc")).unwrap();

        let mut clt = CltTest::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        let mut poll_handler = PollHandlerStatic::default();
        poll_handler.add_acceptor(acceptor);

        let _spawned_poll_handler = poll_handler.into_spawned_handler("Static-Svc-Poll-Thread");

        let mut msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let write_count = 10;
        for _ in 0..write_count {
            clt.send_busywait(&mut msg).unwrap();
        }

        let start = Instant::now();
        while start.elapsed() < setup::net::default_find_timeout() {
            if counter.recv_count() == write_count {
                break;
            }
        }
        assert_eq!(counter.recv_count(), write_count);

        // test that second connection is denied due to svc having set the limit of 1 on max connections
        let mut clt1 = CltTest::connect(
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
        let mut clt1 = CltTest::connect(
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

        let svc1 = SvcTest::bind(addr1, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), SvcTestProtocolManual::default(), Some("unittest/svc1")).unwrap();
        let svc2 = SvcTest::bind(addr2, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), SvcTestProtocolManual::default(), Some("unittest/svc2")).unwrap();

        let clt1 = CltTest::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            StoreCallback::new_ref(store.clone()),
            CltTestProtocolManual::default(),
            Some("unittest/clt1"),
        )
        .unwrap();
        let clt2 = CltTest::connect(
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

        let spawned_poll_handler = poll_handler.into_spawned_handler("Dynamic-Svc/Clt-Poll-Thread");
        // try adding after spawning
        spawned_poll_handler.add_recver(Box::new(clt1_recver));
        spawned_poll_handler.add_recver(Box::new(clt2_recver));

        clt1.send_busywait(&mut CltTestMsgDebug::new(b"Hello From Clt1").into()).unwrap();
        clt2.send_busywait(&mut CltTestMsgDebug::new(b"Hello From Clt2").into()).unwrap();
        svc1.send_busywait(&mut SvcTestMsgDebug::new(b"Hello From Svc1").into()).unwrap();
        svc2.send_busywait(&mut SvcTestMsgDebug::new(b"Hello From Svc2").into()).unwrap();

        let found = store.find_recv("unittest/svc1", |_x| true, setup::net::default_optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Clt(CltTestMsg::Dbg(msg)) if msg == CltTestMsgDebug::new(b"Hello From Clt1")));

        let found = store.find_recv("unittest/svc2", |_x| true, setup::net::default_optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Clt(CltTestMsg::Dbg(msg)) if msg == CltTestMsgDebug::new(b"Hello From Clt2")));

        let found = store.find_recv("unittest/clt1", |_x| true, setup::net::default_optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Svc(SvcTestMsg::Dbg(msg)) if msg == SvcTestMsgDebug::new(b"Hello From Svc1")));

        let found = store.find_recv("unittest/clt2", |_x| true, setup::net::default_optional_find_timeout());
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert!(matches!(found.unwrap(), UniTestMsg::Svc(SvcTestMsg::Dbg(msg)) if msg == SvcTestMsgDebug::new(b"Hello From Svc2")));

        info!("store: {}", store);
    }

    #[test]
    fn test_poller_spawned_double_shutdown_pass() {
        setup::log::configure_level(log::LevelFilter::Info);
        crate::connect::DEFAULT_POLL_HANDLER.shutdown(Some(ConId::default()));
        crate::connect::DEFAULT_POLL_HANDLER.total_shutdown_check();
    }

    #[test]
    #[should_panic(expected = "Invalid API usage. Trying to use SpawnedPollHandler after SpawnedPollHandler::shutdown(None) has been issued.")]
    fn test_poller_spawned_double_shutdown_fail() {
        setup::log::configure_level(log::LevelFilter::Info);
        crate::connect::DEFAULT_POLL_HANDLER.shutdown(None);
        crate::connect::DEFAULT_POLL_HANDLER.total_shutdown_check();
    }

    #[test]
    fn test_poller_spawned_terminate() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr1 = setup::net::rand_avail_addr_port();
        let addr2 = setup::net::rand_avail_addr_port();
        let svc1 = SvcTest::bind(addr1, NonZeroUsize::new(1).unwrap(), DevNullCallback::new_ref(), SvcTestProtocolManual::default(), Some("svc1/unittest"))
            .unwrap()
            .into_sender_with_spawned_recver_ref();
        let svc2_counter_callback = CounterCallback::new_ref();
        let _svc2 = SvcTest::bind(addr2, NonZeroUsize::new(1).unwrap(), svc2_counter_callback.clone(), SvcTestProtocolManual::default(), Some("svc2/unittest"))
            .unwrap()
            .into_sender_with_spawned_recver_ref();
        let mut clt1 = CltTest::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("clt1/unittest"),
        )
        .unwrap()
        .into_sender_with_spawned_recver_ref();
        let mut clt2 = CltTest::connect(
            addr2,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("clt2/unittest"),
        )
        .unwrap()
        .into_sender_with_spawned_recver_ref();

        clt1.send(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello From Clt1"))).unwrap().unwrap_completed();
        // drop to ensure the poll_handler releases all connections associated with the acceptor
        drop(svc1);

        // This loop should terminate quickly as the pool handler should shutdown all svc1 connections including acceptor
        let start = Instant::now();
        loop {
            match clt1.send(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello From Clt1"))) {
                Ok(status) => {
                    log::info!("status: {:?}", status);
                    if start.elapsed() > setup::net::default_connect_timeout() {
                        assert!(
                            false,
                            "Failed to detect that poll handler terminated, which should have shutdown the clt receiver, which in turn should have shutdown the socket for clt sender"
                        );
                    }
                }
                Err(e) => {
                    log::info!("EXPECTED error: {}", e);
                    break;
                }
            }
        }

        // this connection should timeout because Svc1 poll accept has been terminated
        let res = CltTest::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("clt1/fails/unittest"),
        );
        log::info!("res: {:?}", res);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::TimedOut);

        // clt1 should still be functional
        clt2.send(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello From Clt1"))).unwrap().unwrap_completed();
        svc2_counter_callback.assert_recv_count_busywait_timeout(1, setup::net::default_connect_timeout());

        // should be able to restart svc1 on same port
        let svc1_restart_counter_callback = CounterCallback::new_ref();
        let _svc1_restart = SvcTest::bind(addr1, NonZeroUsize::new(1).unwrap(), svc1_restart_counter_callback.clone(), SvcTestProtocolManual::default(), Some("svc1/restart/unittest"))
            .unwrap()
            .into_sender_with_spawned_recver_ref();
        let mut clt1_restart = CltTest::connect(
            addr1,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::new_ref(),
            CltTestProtocolManual::default(),
            Some("clt1/restart/unittest"),
        )
        .unwrap()
        .into_sender_with_spawned_recver_ref();
        clt1_restart.send(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello From Clt1"))).unwrap().unwrap_completed();
        svc1_restart_counter_callback.assert_recv_count_busywait_timeout(1, setup::net::default_connect_timeout());
    }
}
