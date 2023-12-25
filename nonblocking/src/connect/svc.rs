use std::{
    fmt::Display,
    io::Error,
    num::NonZeroUsize,
    os::fd::{FromRawFd, IntoRawFd},
    sync::Arc,
};

use links_core::asserted_short_name;
use log::{debug, log_enabled, warn};

use crate::prelude::*;

use super::{clt::CltRecverRef, pool::TransmittingSvcAcceptorRef};

pub type SvcRecver<P, C, const MAX_MSG_SIZE: usize> = CltRecversPool<P, CltRecver<P, C, MAX_MSG_SIZE>>;
pub type SvcSender<P, C, const MAX_MSG_SIZE: usize> = CltSendersPool<P, CltSender<P, C, MAX_MSG_SIZE>>;

pub type SvcRecverRef<P, C, const MAX_MSG_SIZE: usize> = CltRecversPool<P, CltRecverRef<P, C, MAX_MSG_SIZE>>;
pub type SvcSenderRef<P, C, const MAX_MSG_SIZE: usize> = CltSendersPool<P, CltSenderRef<P, C, MAX_MSG_SIZE>>;

/// Helper class that create [Clt] instances by accepting new connections on a [std::net::TcpListener]
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::SvcTestProtocolManual};
/// use links_core::unittest::setup::{self, messenger::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
///
/// let addr = setup::net::rand_avail_addr_port(); // "127.0.0.1:8080" generates random port
/// let acceptor = SvcAcceptor::<_, _, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::default().into(),
///     SvcTestProtocolManual::default(),
///     NonZeroUsize::new(1).unwrap(),
/// );
///
/// let status = acceptor.accept().unwrap();
/// assert!(status.is_wouldblock());
///
/// ```
#[derive(Debug)]
pub struct SvcAcceptor<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    pub(crate) listener: mio::net::TcpListener,
    acceptor_limiter: AcceptorConnectionGate,
    callback: Arc<C>,
    protocol: P,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SvcAcceptor<P, C, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, listener: std::net::TcpListener, callback: Arc<C>, protocol: P, max_connections: NonZeroUsize) -> Self {
        listener.set_nonblocking(true).expect("Failed to set nonblocking on listener");
        Self {
            con_id,
            listener: mio::net::TcpListener::from_std(listener),
            acceptor_limiter: AcceptorConnectionGate::new(max_connections),
            callback,
            protocol,
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SvcAcceptorOfCltNonBlocking<Clt<P, C, MAX_MSG_SIZE>> for SvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn accept(&self) -> Result<AcceptStatus<Clt<P, C, MAX_MSG_SIZE>>, Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                match self.acceptor_limiter.increment() {
                    Ok(()) => {}
                    Err(e) => {
                        if log_enabled!(log::Level::Warn) {
                            warn!("{} Rejected stream: {:?} due to error: {}", self.con_id, stream, e);
                        }
                        return Ok(AcceptStatus::Rejected);
                    }
                }

                let stream = unsafe { std::net::TcpStream::from_raw_fd(stream.into_raw_fd()) };

                let con_id = {
                    let mut con_id = self.con_id.clone();
                    con_id.set_peer(addr);
                    if log_enabled!(log::Level::Debug) {
                        debug!("{} Accepted", con_id);
                    };
                    con_id
                };
                let acceptor_connection_gate = Some(self.acceptor_limiter.get_new_connection_barrier());
                let clt = Clt::<P, C, MAX_MSG_SIZE>::from_stream(stream, con_id, self.callback.clone(), self.protocol.clone(), acceptor_connection_gate)?;
                Ok(AcceptStatus::Accepted(clt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(AcceptStatus::WouldBlock),
            Err(e) => Err(e),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for SvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for SvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("SvcAcceptor", Self), self.con_id, recv_t, send_t, MAX_MSG_SIZE)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> From<Svc<P, C, MAX_MSG_SIZE>> for SvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn from(svc: Svc<P, C, MAX_MSG_SIZE>) -> Self {
        svc.acceptor
    }
}

/// An abstraction over [MessageRecver] and [MessageSender] that calls a respective callback on every
/// message being processed by internal pool of [Clt]'s managed by [CltsPool]
/// It is designed to work in a single thread.
///
/// * Use [Svc::into_split] to get its parts of [TransmittingSvcAcceptor], [CltRecversPool<_, CltRecver>], [CltSendersPool<_, CltSender>]
/// * Use [Svc::into_split_ref] to get its parts of [TransmittingSvcAcceptorRef], [CltRecversPool<_, CltRecverRef>], [CltSendersPool<_, CltSenderRef>]
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::SvcTestProtocolManual};
/// use links_core::unittest::setup::{self, messenger::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
/// use std::{io::ErrorKind, fmt::Display};
///
///
/// let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
///     setup::net::rand_avail_addr_port(), // 127.0.0.1:8080 generates random port
///     NonZeroUsize::new(1).unwrap(),
///     DevNullCallback::default().into(),
///     SvcTestProtocolManual::default(),
///     Some("doctest"),
/// ).unwrap();
///
/// let status = svc.accept_into_pool().unwrap();
///
/// let err = svc.recv().unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::NotConnected);
/// ```
#[derive(Debug)]
pub struct Svc<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    acceptor: SvcAcceptor<P, C, MAX_MSG_SIZE>,
    clts_pool: CltsPool<P, C, MAX_MSG_SIZE>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Svc<P, C, MAX_MSG_SIZE> {
    /// Binds to a given address and returns an instance [Svc]
    pub fn bind(addr: &str, max_connections: NonZeroUsize, callback: Arc<C>, protocol: P, name: Option<&str>) -> Result<Self, Error> {
        let acceptor = SvcAcceptor::new(ConId::svc(name, addr, None), std::net::TcpListener::bind(addr)?, callback, protocol, max_connections);
        // make pool twice as big as acceptor will allow to be opened this is to ensure that acceptor is able to add new connections to the pool even
        // if some of the connections in the pool are dead but not closed yet
        let pool_size = max_connections.get() * 2;

        let clts_pool = CltsPool::new(acceptor.con_id().clone(), NonZeroUsize::new(pool_size).unwrap());
        Ok(Self { acceptor, clts_pool })
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.clts_pool.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.clts_pool.is_empty()
    }
    #[inline(always)]
    pub fn pool(&self) -> &CltsPool<P, C, MAX_MSG_SIZE> {
        &self.clts_pool
    }
    /// Will split [Svc] into owned [TransmittingSvcAcceptor], [SvcRecver] and [SvcSender]
    ///
    /// # Important
    /// These parts will support only 'subset' of [Protocol] features which are part of [crate::prelude::ProtocolCore] trait
    pub fn into_split(self) -> (TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE>, SvcRecver<P, C, MAX_MSG_SIZE>, SvcSender<P, C, MAX_MSG_SIZE>) {
        let ((tx_recver, tx_sender), (svc_recver, svc_sender)) = self.clts_pool.into_split();
        let acceptor = TransmittingSvcAcceptor::new(tx_recver, tx_sender, self.acceptor);
        (acceptor, svc_recver, svc_sender)
    }
    /// Will split [Svc] into owned [TransmittingSvcAcceptorRef], [SvcRecverRef] and [SvcSenderRef]
    ///
    /// # Important
    /// These parts will support `all` [Protocol] features, which means that `ref counted clone` of [CltRecverRef] will be returned,
    /// while another `ref counted clone` will be moved to run in the [static@crate::connect::DEFAULT_HBEAT_HANDLER] thread
    pub fn into_split_ref(self) -> (TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE>, SvcRecverRef<P, C, MAX_MSG_SIZE>, SvcSenderRef<P, C, MAX_MSG_SIZE>) {
        let ((tx_recver, tx_sender), (svc_recver, svc_sender)) = self.clts_pool.into_split_ref();
        let acceptor = TransmittingSvcAcceptorRef::new(tx_recver, tx_sender, self.acceptor);
        (acceptor, svc_recver, svc_sender)
    }

    /// Will split using [`Self::into_split()`] and only return [SvcSender] while moving [TransmittingSvcAcceptor] to run in the [static@crate::connect::DEFAULT_POLL_HANDLER] thread
    ///
    /// # Important
    /// Please note [`Self::into_split()`] will support only 'subset' of [Protocol] features which are part of [crate::prelude::ProtocolCore] trait
    ///
    /// # Warning
    /// This method will have to `drop` any open [SvcRecver] connections since any [SvcRecver] connections accepted from this point on will
    /// have to be managed by [static@crate::connect::DEFAULT_POLL_HANDLER] thread.
    ///
    /// To mitigate `drop` this call will `panic` if the instance accepted any connections prior to calling this method.
    /// To avoid `panic` call this immediately after creating [Svc] instance
    pub fn into_sender_with_spawned_recver(self) -> SvcSender<P, C, MAX_MSG_SIZE> {
        if !self.clts_pool.is_empty() {
            panic!(
                "
            Invalid API usage. 
            Can't call Svc::into_sender_with_spawned_recver after it established its first connection.
            Current connection pool: {}
            ",
                self.clts_pool
            )
        }
        let (acceptor, _recver_drop, sender) = self.into_split();
        crate::connect::DEFAULT_POLL_HANDLER.add_acceptor(acceptor.into());
        sender
    }

    /// Will split using [`Self::into_split_ref()`] and only return [SvcSenderRef] while moving [TransmittingSvcAcceptorRef] to run in the [static@crate::connect::DEFAULT_POLL_HANDLER] thread
    ///
    /// # Important
    /// Please note [`Self::into_split_ref()`] will support `all` [Protocol] features, which means that `ref counted clone` of [CltRecverRef] will be returned,
    /// while another `ref counted clone` will be moved to run in the [static@crate::connect::DEFAULT_HBEAT_HANDLER] thread
    ///
    /// # Warning
    /// This method `drops` [SvcRecverRef], as a result this call will panic if the instance accepted connections prior to calling this method.
    /// To avoid this call this immediately after creating [Svc] instance and prior to accepting any connections
    pub fn into_sender_with_spawned_recver_ref(self) -> SvcSenderRef<P, C, MAX_MSG_SIZE> {
        if !self.clts_pool.is_empty() {
            panic!(
                "
            Invalid API usage. 
            Can't call Svc::into_sender_with_spawned_recver_ref after it established its first connection.
            Current connection pool: {}
            ",
                self.clts_pool
            )
        }
        let (acceptor, _recver_drop, sender) = self.into_split_ref();
        crate::connect::DEFAULT_POLL_HANDLER.add_acceptor(acceptor.into());

        sender
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolSvcAcceptorOfCltNonBlocking for Svc<P, C, MAX_MSG_SIZE> {
    /// Will attempt to accept a new connection and add it to the pool. If the pool is full it will return an [std::io::ErrorKind::OutOfMemory].
    fn accept_into_pool(&mut self) -> Result<PoolAcceptStatus, Error> {
        match self.acceptor.accept()? {
            AcceptStatus::Accepted(clt) => {
                self.clts_pool.add(clt)?;
                Ok(PoolAcceptStatus::Accepted)
            }
            AcceptStatus::Rejected => Ok(PoolAcceptStatus::Rejected),
            AcceptStatus::WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SvcAcceptorOfCltNonBlocking<Clt<P, C, MAX_MSG_SIZE>> for Svc<P, C, MAX_MSG_SIZE> {
    /// Instead of adding the accepted connection to the pool it will return it to the caller.
    fn accept(&self) -> Result<AcceptStatus<Clt<P, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P::SendT> for Svc<P, C, MAX_MSG_SIZE> {
    /// Will use the underling [CltsPool] to deliver the message to one of the [Clt]'s in the pool.

    #[inline(always)]
    fn send(&mut self, msg: &mut P::SendT) -> Result<SendStatus, Error> {
        self.clts_pool.send(msg)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P::RecvT> for Svc<P, C, MAX_MSG_SIZE> {
    /// Will use the underling [CltsPool] to receive a message from one of the [Clt]'s in the pool.
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        self.clts_pool.recv()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for Svc<P, C, MAX_MSG_SIZE> {
    fn con_id(&self) -> &ConId {
        self.acceptor.con_id()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolConnectionStatus for Svc<P, C, MAX_MSG_SIZE> {
    /// Will delegate to [`CltsPool::is_next_connected()`]
    #[inline(always)]
    fn is_next_connected(&mut self) -> bool {
        self.clts_pool.is_next_connected()
    }
    /// Will delegate to [`CltsPool::all_connected()`]
    #[inline(always)]
    fn all_connected(&mut self) -> bool {
        self.clts_pool.all_connected()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for Svc<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}, {}>", asserted_short_name!("Svc", Self), self.acceptor, self.clts_pool)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::{
        prelude::*,
        unittest::setup::protocol::{CltTestProtocolAuthAndHbeat, CltTestProtocolManual, SvcTestProtocolAuthAndHBeat, SvcTestProtocolManual},
    };
    use links_core::unittest::setup::{
        self,
        framer::TEST_MSG_FRAME_SIZE,
        model::{CltTestMsg, CltTestMsgDebug, CltTestMsgLoginReq, SvcTestMsg, SvcTestMsgDebug},
    };
    use log::{info, Level, LevelFilter};
    use rand::Rng;
    use std::{
        io::ErrorKind,
        num::NonZeroUsize,
        thread::Builder,
        time::{Duration, Instant},
    };

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallback::new_ref();
        let protocol = SvcTestProtocolManual::default();
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(2).unwrap(), callback.clone(), protocol, Some("unittest")).unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.pool().len(), 0);
    }

    #[test]
    fn test_svc_clt_connected_max_connection() {
        setup::log::configure_compact(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Info);
        let protocol = SvcTestProtocolManual::default();
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver();
        info!("svc: {}", svc);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolManual::default();
        let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver();
        info!("clt: {}", clt);

        // ********** change protocol ot auth and hbeat so that on_connect is called and err is detected due to max_connect **********
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolAuthAndHbeat::default();
        // second connection should fail
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest"));
        info!("res: {:?}", res);
        assert!(res.is_err());

        drop(clt);

        // after dropping the first connection the second connection should succeed
        let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
        info!("clt: {}", clt);
    }

    #[test]
    fn test_svc_clt_connected_not_split_clt_drop() {
        setup::log::configure_compact(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Info);
        let protocol = SvcTestProtocolManual::default();
        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolManual::default();
        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
        info!("clt: {}", clt);

        assert!(!svc.is_next_connected());
        assert!(!svc.all_connected());

        svc.accept_into_pool_busywait().unwrap();

        assert!(svc.is_next_connected());
        assert!(svc.all_connected());

        info!("svc: {}", svc);
        assert_eq!(svc.len(), 1);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));

        clt.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc.recv_busywait().unwrap().unwrap();
        info!("clt_msg_inp: {:?}", clt_msg_inp);
        info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        svc.send_busywait(&mut svc_msg_inp).unwrap();
        let clt_msg_out = clt.recv_busywait().unwrap().unwrap();
        info!("svc_msg_inp: {:?}", svc_msg_inp);
        info!("clt_msg_out: {:?}", clt_msg_out);
        assert_eq!(svc_msg_inp, clt_msg_out);

        // test that second connection is denied due to svc having set the limit of 1 on max connections
        assert!(svc.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap().is_wouldblock()); // make sure pool connection is ejected if no longer working
        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
        svc.accept_into_pool_busywait().unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_completed_none());
        drop(clt);

        // however after dropping clt a new connection can be established, drop will close the socket which svc will detect and allow a new connection
        assert!(svc.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap().is_completed_none()); // make sure pool connection is ejected if no longer working
        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
        svc.accept_into_pool_busywait().unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_wouldblock());
    }

    #[test]
    fn test_scv_clt_connected_not_split_svc_drop() {
        setup::log::configure_compact(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Info);
        let protocol = SvcTestProtocolAuthAndHBeat::default();
        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let clt_jh = Builder::new()
            .name("Clt-Thread".to_owned())
            .spawn(move || {
                let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
                let protocol = CltTestProtocolManual::default();
                let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
                info!("clt: {}", clt);
                let timeout = Duration::from_millis(100);
                clt.send_busywait_timeout(&mut CltTestMsgLoginReq::default().into(), timeout).unwrap();
                let msg = clt.recv_busywait_timeout(timeout).unwrap();
                assert!(matches!(msg, RecvStatus::Completed(Some(SvcTestMsg::Accept(_)))));
                clt.recv_busywait_timeout(timeout).unwrap()
            })
            .unwrap();

        svc.accept_into_pool_busywait().unwrap();

        drop(svc);
        let status = clt_jh.join().unwrap();
        info!("status: {:?}", status);
        assert!(matches!(status, RecvStatus::Completed(Some(SvcTestMsg::Final(_)))));
    }

    #[test]
    fn test_svc_clt_connected_split_clt_drop() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = SvcTestProtocolManual::default();
        let (mut svc_acceptor, mut svc_pool_recver, mut svc_pool_sender) = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap().into_split();
        info!("svc_acceptor: {}", svc_acceptor);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolManual::default();
        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, protocol, Some("unittest")).unwrap();
        info!("clt: {}", clt);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));

        assert!(!svc_pool_recver.all_connected());
        assert!(!svc_pool_recver.is_next_connected());
        assert!(!svc_pool_sender.all_connected());
        assert!(!svc_pool_sender.is_next_connected());

        svc_acceptor.accept_into_pool_busywait().unwrap();

        assert!(svc_pool_recver.all_connected());
        assert!(svc_pool_recver.is_next_connected());
        assert!(svc_pool_sender.all_connected());
        assert!(svc_pool_sender.is_next_connected());

        info!("--------- CLT PRE-SPLIT ---------");
        clt.send_busywait(&mut clt_msg_inp).unwrap();

        let svc_msg_out = svc_pool_recver.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        let (mut clt_recv, mut clt_send) = clt.into_split();

        info!("--------- CLT SPLIT ---------");
        clt_send.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc_pool_recver.recv_busywait().unwrap().unwrap();
        // info!("svc_msg_out: {:?}", svc_msg_out);
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        assert_eq!(svc_msg_out, clt_msg_inp);

        info!("--------- CLT DROP RANDOM HALF ---------");
        // drop clt_recv and ensure that clt_sender has broken pipe
        let drop_send = rand::thread_rng().gen_range(1..=2) % 2 == 0;

        if drop_send {
            info!("dropping clt_send");
            drop(clt_send);
            let status = clt_recv.recv().unwrap();
            info!("clt_recv status: {:?}", status);
            assert!(status.is_completed_none());
        } else {
            info!("dropping clt_recv");
            drop(clt_recv); // drop of recv shuts down Write half of cloned stream and hence impacts clt_send
            let err = clt_send.send(&mut clt_msg_inp).unwrap_err();
            info!("clt_send err: {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }

        info!("--------- SVC RECV/SEND SHOULD FAIL CLT DROPS HALF ---------");
        // recv with busywait to ensure that clt drop has delivered FIN signal and receiver does not just return WouldBlock
        let status = svc_pool_recver.recv_busywait_timeout(Duration::from_millis(100)).unwrap();
        info!("pool_recver status: {:?}", status);
        assert!(status.is_completed_none());
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let res = svc_pool_sender.send(&mut svc_msg_inp);
        info!("pool_sender res: {:?}", res);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), ErrorKind::BrokenPipe);
    }

    #[test]
    fn test_svc_clt_connected_split_svc_drop() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = SvcTestProtocolAuthAndHBeat::default();
        let (mut svc_acceptor, _svc_pool_recver, svc_pool_sender) = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap().into_split();
        info!("svc_acceptor: {}", svc_acceptor);

        let clt_jh = Builder::new()
            .name("Clt-Thread".to_owned())
            .spawn(move || {
                let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
                let protocol = CltTestProtocolManual::default();
                let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
                info!("clt: {}", clt);
                let timeout = Duration::from_millis(100);
                clt.send_busywait_timeout(&mut CltTestMsgLoginReq::default().into(), timeout).unwrap();
                let msg = clt.recv_busywait_timeout(timeout).unwrap();
                assert!(matches!(msg, RecvStatus::Completed(Some(SvcTestMsg::Accept(_)))));
                clt.recv_busywait_timeout(timeout).unwrap()
            })
            .unwrap();

        svc_acceptor.accept_into_pool_busywait().unwrap();

        drop(svc_pool_sender);
        let status = clt_jh.join().unwrap();
        info!("status: {:?}", status);
        assert!(matches!(status, RecvStatus::Completed(Some(SvcTestMsg::Final(_)))));
    }

    #[test]
    fn test_svc_clt_connected_split_ref_clt_drop() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = SvcTestProtocolManual::default();
        let (mut svc_acceptor, mut svc_pool_recver, mut svc_pool_sender) = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap().into_split_ref();
        info!("svc_acceptor: {}", svc_acceptor);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolManual::default();
        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, protocol, Some("unittest")).unwrap();
        info!("clt: {}", clt);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));

        assert!(!svc_pool_recver.all_connected());
        assert!(!svc_pool_recver.is_next_connected());
        assert!(!svc_pool_sender.all_connected());
        assert!(!svc_pool_sender.is_next_connected());

        svc_acceptor.accept_into_pool_busywait().unwrap();

        assert!(svc_pool_recver.all_connected());
        assert!(svc_pool_recver.is_next_connected());
        assert!(svc_pool_sender.all_connected());
        assert!(svc_pool_sender.is_next_connected());

        info!("--------- CLT PRE-SPLIT ---------");
        clt.send_busywait(&mut clt_msg_inp).unwrap();

        let svc_msg_out = svc_pool_recver.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        let (mut clt_recv, mut clt_send) = clt.into_split_ref();

        info!("--------- CLT SPLIT ---------");
        clt_send.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc_pool_recver.recv_busywait().unwrap().unwrap();
        // info!("svc_msg_out: {:?}", svc_msg_out);
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        assert_eq!(svc_msg_out, clt_msg_inp);

        info!("--------- CLT DROP RANDOM HALF ---------");
        // drop clt_recv and ensure that clt_sender has broken pipe
        let drop_send = rand::thread_rng().gen_range(1..=2) % 2 == 0;
        if drop_send {
            info!("dropping clt_send");
            drop(clt_send);
            let status = clt_recv.recv().unwrap();
            info!("clt_recv status: {:?}", status);
            assert!(status.is_completed_none());
        } else {
            info!("dropping clt_recv");
            drop(clt_recv); // drop of recv shuts down Write half of cloned stream and hence impacts clt_send
            let err = clt_send.send(&mut clt_msg_inp).unwrap_err();
            info!("clt_send err: {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }

        info!("--------- SVC RECV/SEND SHOULD FAIL CLT DROPS clt_recv ---------");
        // recv with busywait to ensure that clt drop has delivered FIN signal and receiver does not just return WouldBlock
        let status = svc_pool_recver.recv_busywait_timeout(Duration::from_millis(100)).unwrap();
        info!("pool_recver opt: {:?}", status);
        assert!(status.is_completed_none());
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let err = svc_pool_sender.send(&mut svc_msg_inp).unwrap_err();
        info!("pool_sender err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }

    #[test]
    fn test_svc_clt_connected_split_ref_svc_drop() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = SvcTestProtocolAuthAndHBeat::default();
        let (mut svc_acceptor, _svc_pool_recver, svc_pool_sender) = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), callback, protocol, Some("unittest")).unwrap().into_split_ref();
        // info!("svc_acceptor: {}", svc_acceptor);

        let clt_jh = Builder::new()
            .name("Clt-Thread".to_owned())
            .spawn(move || {
                let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
                let protocol = CltTestProtocolManual::default();
                let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), protocol.clone(), Some("unittest")).unwrap();
                // info!("clt: {}", clt);
                let timeout = Duration::from_millis(100);
                clt.send_busywait_timeout(&mut CltTestMsgLoginReq::default().into(), timeout).unwrap();
                let msg = clt.recv_busywait_timeout(timeout).unwrap();
                assert!(matches!(msg, RecvStatus::Completed(Some(SvcTestMsg::Accept(_)))));
                let now = Instant::now();
                loop {
                    let status = clt.recv_busywait_timeout(timeout).unwrap();
                    if let RecvStatus::Completed(Some(SvcTestMsg::Final(_))) = status {
                        return status;
                    }
                    if now.elapsed() > timeout {
                        panic!("Timeout waiting for Final");
                    }
                }
            })
            .unwrap();

        svc_acceptor.accept_into_pool_busywait().unwrap();

        drop(svc_pool_sender);
        let status = clt_jh.join().unwrap();
        info!("status: {:?}", status);
        assert!(matches!(status, RecvStatus::Completed(Some(SvcTestMsg::Final(_)))));
    }

    #[test]
    fn test_svc_clt_connected_spawned_recver() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let clt_count = CounterCallback::new_ref();
        let svc_count = CounterCallback::new_ref();
        let clt_clbk = ChainCallback::new_ref(vec![LoggerCallback::with_level_ref(log::Level::Info, log::Level::Debug), clt_count.clone()]);
        let svc_clbk = ChainCallback::new_ref(vec![LoggerCallback::with_level_ref(log::Level::Info, log::Level::Debug), svc_count.clone()]);
        let io_timeout = setup::net::default_io_timeout();

        let protocol = SvcTestProtocolManual::default();
        let mut svc_sender = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), svc_clbk, protocol, Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver();

        let protocol = CltTestProtocolManual::default();
        let mut clt_sender = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), clt_clbk, protocol, Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver();

        info!("clt_sender.is_connected(): {}", clt_sender.is_connected());
        info!("svc_sender.all_connected(): {}", svc_sender.all_connected());

        info!("clt_count: {}", clt_count);
        assert_eq!(clt_count.sent_count(), 0);
        info!("svc_count: {}", svc_count);
        assert_eq!(svc_count.sent_count(), 0);

        const N: usize = 10;
        for i in 1..=N {
            clt_sender.send_busywait_timeout(&mut CltTestMsgDebug::new(format!("Msg  #{}", i).as_bytes()).into(), io_timeout).unwrap().unwrap_completed();
        }
        assert_eq!(svc_count.recv_count_busywait_timeout(N, setup::net::find_timeout()), N);
        info!("scv_count: {}", svc_count);
        assert_eq!(svc_count.sent_count(), 0);
        assert_eq!(clt_count.sent_count(), N);

        for i in 1..=N {
            svc_sender.send_busywait_timeout(&mut SvcTestMsgDebug::new(format!("Msg  #{}", i).as_bytes()).into(), io_timeout).unwrap().unwrap_completed();
        }
        assert_eq!(clt_count.recv_count_busywait_timeout(N, setup::net::find_timeout()), N);
        info!("clt_count: {}", clt_count);
        assert_eq!(svc_count.sent_count(), N);
        assert_eq!(clt_count.sent_count(), N);
    }

    #[test]
    fn test_svc_clt_connected_spawned_recver_ref() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let clt_count = CounterCallback::new_ref();
        let svc_count = CounterCallback::new_ref();
        let clt_clbk = ChainCallback::new_ref(vec![LoggerCallback::with_level_ref(log::Level::Info, log::Level::Debug), clt_count.clone()]);
        let svc_clbk = ChainCallback::new_ref(vec![LoggerCallback::with_level_ref(log::Level::Info, log::Level::Debug), svc_count.clone()]);
        let io_timeout = setup::net::default_io_timeout();

        let protocol = SvcTestProtocolManual::default();
        let mut svc_sender = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), svc_clbk, protocol, Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver_ref();

        let protocol = CltTestProtocolManual::default();
        let mut clt_sender = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), clt_clbk, protocol, Some("unittest"))
            .unwrap()
            .into_sender_with_spawned_recver_ref();

        let clt_connected = clt_sender.is_connected();
        let svc_connected = svc_sender.all_connected_busywait_timeout(setup::net::find_timeout());
        info!("clt_sender.is_connected(): {}", clt_connected);
        info!("svc_sender.all_connected(): {}", svc_connected);
        assert!(clt_connected);
        assert!(svc_connected);

        info!("clt_count: {}", clt_count);
        assert_eq!(clt_count.sent_count(), 0);
        info!("svc_count: {}", svc_count);
        assert_eq!(svc_count.sent_count(), 0);

        const N: usize = 10;
        for i in 1..=N {
            clt_sender.send_busywait_timeout(&mut CltTestMsgDebug::new(format!("Msg  #{}", i).as_bytes()).into(), io_timeout).unwrap().unwrap_completed();
        }

        assert_eq!(svc_count.recv_count_busywait_timeout(N, setup::net::find_timeout()), N);
        info!("scv_count: {}", svc_count);
        assert_eq!(svc_count.sent_count(), 0);
        assert_eq!(clt_count.sent_count(), N);

        for i in 1..=N {
            svc_sender.send_busywait_timeout(&mut SvcTestMsgDebug::new(format!("Msg  #{}", i).as_bytes()).into(), io_timeout).unwrap().unwrap_completed();
        }
        assert_eq!(clt_count.recv_count_busywait_timeout(N, io_timeout), N);
        info!("clt_count: {}", clt_count);
        assert_eq!(svc_count.sent_count(), N);
        assert_eq!(clt_count.sent_count(), N);
    }
}
