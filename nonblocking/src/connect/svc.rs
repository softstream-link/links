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

/// Helper class that create [Clt] instances by accepting new connections on a [std::net::TcpListener]
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::SvcTestProtocolAuth};
/// use links_core::unittest::setup::{self, messenger::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
///
/// let addr = setup::net::rand_avail_addr_port(); // "127.0.0.1:8080" generates random port
/// let acceptor = SvcAcceptor::<_, _, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::default().into(),
///     Some(SvcTestProtocolAuth::default()),
///     NonZeroUsize::new(1).unwrap(),
/// );
///
/// let status = acceptor.accept().unwrap();
/// assert!(status.is_wouldblock());
///
/// ```
#[derive(Debug)]
pub struct SvcAcceptor<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) listener: mio::net::TcpListener,
    acceptor_limiter: AcceptorConnectionGate,
    callback: Arc<C>,
    protocol: Option<P>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SvcAcceptor<P, C, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, listener: std::net::TcpListener, callback: Arc<C>, protocol: Option<P>, max_connections: NonZeroUsize) -> Self {
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
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> AcceptNonBlocking<Clt<P, C, MAX_MSG_SIZE>> for SvcAcceptor<P, C, MAX_MSG_SIZE> {
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
/// It is designed to work in a single thread. To split out [CltRecversPool], [CltSendersPool] and [SvcPoolAcceptor] use [Svc::into_split]
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::SvcTestProtocolAuth};
/// use links_core::unittest::setup::{self, messenger::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
/// use std::{io::ErrorKind, fmt::Display};
///
///
/// let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
///     setup::net::rand_avail_addr_port(), // 127.0.0.1:8080 generates random port
///     DevNullCallback::default().into(),
///     NonZeroUsize::new(1).unwrap(),
///     Some(SvcTestProtocolAuth::default()),
///     Some("doctest"),
/// ).unwrap();
///
/// let status = svc.pool_accept().unwrap();
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
    pub fn bind(addr: &str, callback: Arc<C>, max_connections: NonZeroUsize, protocol: Option<P>, name: Option<&str>) -> Result<Self, Error> {
        let acceptor = SvcAcceptor::new(ConId::svc(name, addr, None), std::net::TcpListener::bind(addr)?, callback, protocol, max_connections);
        // make pool twice as big as acceptor will allow to be opened this is to ensure that acceptor is able to add new connections to the pool even 
        // if some of the connections in the pool are dead but not closed yet
        let pool_size = max_connections.get() * 2;
        let clts_pool = CltsPool::<P, C, MAX_MSG_SIZE>::with_capacity(NonZeroUsize::new(pool_size).unwrap());
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
    /// Will split [Svc] into owned [SvcPoolAcceptor], [CltRecversPool] and [CltSendersPool] all of which can be used by different threads
    pub fn into_split(self) -> (SvcPoolAcceptor<P, C, MAX_MSG_SIZE>, CltRecversPool<P, C, MAX_MSG_SIZE>, CltSendersPool<P, C, MAX_MSG_SIZE>) {
        if !self.clts_pool.is_empty() {
            panic!("Can't call Svc::into_split can Svc already has accepted connections in the pool: {}", self.clts_pool)
        }
        let ((tx_recver, tx_sender), (svc_recver, svc_sender)) = self.clts_pool.into_split();
        let acceptor = SvcPoolAcceptor::new(tx_recver, tx_sender, self.acceptor);
        (acceptor, svc_recver, svc_sender)
    }

    /// Will take [Svc] split it using [Self::into_split] and only return [CltSendersPool] while registering resulting [SvcPoolAcceptor] with
    /// [static@crate::connect::DEFAULT_POLL_HANDLER]
    pub fn into_spawned_sender(self) -> CltSendersPool<P, C, MAX_MSG_SIZE> {
        let (acceptor, _recver_drop, sender) = self.into_split();
        crate::connect::DEFAULT_POLL_HANDLER.add_acceptor(acceptor.into());
        sender
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolAcceptCltNonBlocking for Svc<P, C, MAX_MSG_SIZE> {
    /// Will attempt to accept a new connection and add it to the pool. If the pool is full it will return an [std::io::ErrorKind::OutOfMemory].
    fn pool_accept(&mut self) -> Result<PoolAcceptStatus, Error> {
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
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> AcceptNonBlocking<Clt<P, C, MAX_MSG_SIZE>> for Svc<P, C, MAX_MSG_SIZE> {
    /// Instead of adding the accepted connection to the pool it will return it to the caller.
    fn accept(&self) -> Result<AcceptStatus<Clt<P, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for Svc<P, C, MAX_MSG_SIZE> {
    /// Will use the underling [CltsPool] to deliver the message to one of the [Clt]'s in the pool.
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clts_pool.send(msg)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for Svc<P, C, MAX_MSG_SIZE> {
    /// Will use the underling [CltsPool] to receive a message from one of the [Clt]'s in the pool.
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        self.clts_pool.recv()
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
    use std::{io::ErrorKind, num::NonZeroUsize, time::Duration};

    use crate::{
        prelude::*,
        unittest::setup::protocol::{CltTestProtocolSupervised, SvcTestProtocolSupervised},
    };
    use links_core::unittest::setup::{
        self,
        framer::TEST_MSG_FRAME_SIZE,
        model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg, SvcTestMsgDebug},
    };

    use log::{info, Level, LevelFilter};
    use rand::Rng;

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallback::new_ref();
        let protocol = SvcTestProtocolSupervised::default();
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback.clone(), NonZeroUsize::new(2).unwrap(), Some(protocol), Some("unittest")).unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.pool().len(), 0);
    }

    #[test]
    fn test_svc_clt_connected_not_split() {
        setup::log::configure_compact(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Info);
        let protocol = SvcTestProtocolSupervised::default();
        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback, NonZeroUsize::new(1).unwrap(), Some(protocol), Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolSupervised::default();
        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), Some(protocol.clone()), Some("unittest")).unwrap();
        info!("clt: {}", clt);

        svc.pool_accept_busywait().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.len(), 1);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));
        // info!("--------- PRE SPLIT ---------");
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
        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), Some(protocol.clone()), Some("unittest")).unwrap();
        svc.pool_accept_busywait().unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_completed_none());
        drop(clt);

        // however after dropping clt a new connection can be established, drop will close the socket which svc will detect and allow a new connection
        assert!(svc.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap().is_completed_none()); // make sure pool connection is ejected if no longer working
        let mut clt1 = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback.clone(), Some(protocol.clone()), Some("unittest")).unwrap();
        svc.pool_accept_busywait().unwrap();
        let status = clt1.recv_busywait_timeout(setup::net::default_connect_timeout()).unwrap();
        info!("status: {:?}", status);
        assert!(status.is_wouldblock());
    }

    #[test]
    fn test_svc_clt_connected_split() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = SvcTestProtocolSupervised::default();
        let (mut svc_acceptor, mut svc_pool_recver, mut svc_pool_sender) = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback, NonZeroUsize::new(1).unwrap(), Some(protocol), Some("unittest")).unwrap().into_split();
        info!("svc_acceptor: {}", svc_acceptor);

        let callback = LoggerCallback::with_level_ref(Level::Info, Level::Debug);
        let protocol = CltTestProtocolSupervised::default();
        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, Some(protocol), Some("unittest")).unwrap();
        info!("clt: {}", clt);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));

        svc_acceptor.pool_accept_busywait().unwrap();

        clt.send_busywait(&mut clt_msg_inp).unwrap();

        let svc_msg_out = svc_pool_recver.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- CLT SPLIT DIRECT ---------");
        let (mut clt_recv, mut clt_send) = clt.into_split();
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
        info!("pool_recver opt: {:?}", status);
        assert!(status.is_completed_none());
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let err = svc_pool_sender.send(&mut svc_msg_inp).unwrap_err();
        info!("pool_sender err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
}
