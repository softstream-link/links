use std::{
    fmt::Display,
    io::Error,
    num::NonZeroUsize,
    os::fd::{FromRawFd, IntoRawFd},
    sync::Arc,
};

use links_core::asserted_short_name;
use log::{debug, log_enabled};

use crate::prelude::*;

/// Helper class that create [Clt] instances by accepting new connections on a [std::net::TcpListener]
///
/// # Example
/// ```
/// use links_nonblocking::prelude::*;
/// use links_core::unittest::setup::messenger::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
///
/// let addr = "127.0.0.1:8080";
/// let acceptor = SvcAcceptor::<_, _, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::<SvcTestMessenger>::default().into(),
/// );
///
/// let status = acceptor.accept_nonblocking().unwrap();
/// assert!(status.is_wouldblock());
///
/// ```
#[derive(Debug)]
pub struct SvcAcceptor<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) listener: mio::net::TcpListener,
    callback: Arc<C>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    pub fn new(con_id: ConId, listener: std::net::TcpListener, callback: Arc<C>) -> Self {
        listener
            .set_nonblocking(true)
            .expect("Failed to set nonblocking on listener");
        Self {
            con_id,
            listener: mio::net::TcpListener::from_std(listener),
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // TODO add rate limiter
                let stream = unsafe { std::net::TcpStream::from_raw_fd(stream.into_raw_fd()) };

                let con_id = {
                    let mut con_id = self.con_id.clone();
                    con_id.set_peer(addr);
                    if log_enabled!(log::Level::Debug) {
                        debug!("{} Accepted", con_id);
                    };
                    con_id
                };
                let clt =
                    Clt::<M, C, MAX_MSG_SIZE>::from_stream(stream, con_id, self.callback.clone());
                Ok(AcceptStatus::Accepted(clt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(AcceptStatus::WouldBlock),
            Err(e) => Err(e),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}>",
            asserted_short_name!("SvcAcceptor", Self),
            self.con_id
        )
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> From<Svc<M, C, MAX_MSG_SIZE>>
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn from(svc: Svc<M, C, MAX_MSG_SIZE>) -> Self {
        svc.acceptor
    }
}

/// An abstraction over [MessageRecver] and [MessageSender] that calls a respective callback on every
/// message being processed by internal pool of [Clt]'s managed by [CltsPool]
/// It is designed to work in a single thread. To split out [CltRecversPool], [CltSendersPool] and [PoolCltAcceptor] use [Svc::into_split]
/// # Example
/// ```
/// use links_nonblocking::prelude::*;
/// use links_core::unittest::setup::messenger::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
/// use std::{io::ErrorKind, fmt::Display};
/// let addr = "127.0.0.1:8080";
/// let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
///     addr,
///     DevNullCallback::<SvcTestMessenger>::default().into(),
///     NonZeroUsize::new(1).unwrap(),
///     Some("doctest"),
/// ).unwrap();
///
/// let status = svc.pool_accept_nonblocking().unwrap();
///
/// let err = svc.recv_nonblocking().unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::NotConnected);
/// ```
#[derive(Debug)]
pub struct Svc<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
    clts_pool: CltsPool<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Svc<M, C, MAX_MSG_SIZE> {
    /// Binds to a given address and returns an instance [Svc]
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: NonZeroUsize, // TODO this arg needs better name
        name: Option<&str>,
    ) -> Result<Self, Error> {
        let acceptor = SvcAcceptor::new(
            ConId::svc(name, addr, None),
            std::net::TcpListener::bind(addr)?,
            callback,
        );

        let clts_pool = CltsPool::<M, C, MAX_MSG_SIZE>::with_capacity(max_connections);
        Ok(Self {
            acceptor,
            clts_pool,
        })
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.clts_pool.len()
    }
    #[inline(always)]
    pub fn pool(&self) -> &CltsPool<M, C, MAX_MSG_SIZE> {
        &self.clts_pool
    }
    pub fn into_split(
        self,
    ) -> (
        PoolCltAcceptor<M, C, MAX_MSG_SIZE>,
        CltRecversPool<M, C, MAX_MSG_SIZE>,
        CltSendersPool<M, C, MAX_MSG_SIZE>,
    ) {
        let ((tx_recver, tx_sender), (svc_recver, svc_sender)) = self.clts_pool.into_split();
        let acceptor = PoolCltAcceptor::new(tx_recver, tx_sender, self.acceptor);
        (acceptor, svc_recver, svc_sender)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> PoolAcceptCltNonBlocking
    for Svc<M, C, MAX_MSG_SIZE>
{
    /// Will attempt to accept a new connection and add it to the pool. If the pool is full it will return an error.
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error> {
        match self.acceptor.accept()? {
            AcceptStatus::Accepted(clt) => {
                self.clts_pool.add(clt)?;
                Ok(PoolAcceptStatus::Accepted)
            }
            AcceptStatus::WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    /// Instead of adding the accepted connection to the pool it will return it to the caller.
    fn accept(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    /// Will use the underling [CltsPool] to deliver the message to one of the [Clt]'s in the pool.
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clts_pool.send(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    /// Will use the underling [CltsPool] to receive a message from one of the [Clt]'s in the pool.
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.clts_pool.recv()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}, {}>",
            asserted_short_name!("Svc", Self),
            self.acceptor,
            self.clts_pool
        )
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {
    use std::{io::ErrorKind, num::NonZeroUsize, time::Duration};

    use crate::prelude::*;
    use links_core::unittest::setup::{
        self,
        framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
    };

    use log::{info, Level, LevelFilter};
    use rand::Rng;

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallback::<SvcTestMessenger>::new_ref();
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            callback.clone(),
            NonZeroUsize::new(2).unwrap(),
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.pool().len(), 0);
    }

    #[test]
    fn test_svc_clt_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<SvcTestMessenger>::with_level_ref(Level::Info, Level::Debug),
            NonZeroUsize::new(1).unwrap(),
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            LoggerCallback::<CltTestMessenger>::with_level_ref(Level::Info, Level::Debug),
            Some("unittest"),
        )
        .unwrap();
        info!("clt: {}", clt);

        svc.pool_accept_busywait().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.len(), 1);

        let mut clt_msg_inp = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
        info!("--------- PRE SPLIT ---------");
        clt.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- SVC SPLIT POOL ---------");
        let (_svc_acceptor, mut svc_pool_recver, mut svc_pool_sender) = svc.into_split();
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
            let opt = clt_recv.recv().unwrap().unwrap_completed();
            info!("clt_recv opt: {:?}", opt);
            assert_eq!(opt, None);
        } else {
            info!("dropping clt_recv");
            drop(clt_recv); // drop of recv shuts down Write half of cloned stream and hence impacts clt_send
            let err = clt_send.send(&mut clt_msg_inp).unwrap_err();
            info!("clt_send err: {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }

        info!("--------- SVC RECV/SEND SHOULD FAIL CLT DROPS HALF ---------");
        // recv with busywait to ensure that clt drop has delivered FIN signal and receiver does not just return WouldBlock
        let opt = svc_pool_recver
            .recv_busywait_timeout(Duration::from_millis(100))
            .unwrap()
            .unwrap_completed();
        info!("pool_recver opt: {:?}", opt);
        assert_eq!(opt, None);
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let err = svc_pool_sender
            .send(&mut svc_msg_inp)
            .unwrap_err();
        info!("pool_sender err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
}
