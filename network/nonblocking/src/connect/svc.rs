use std::{
    fmt::Display,
    io::Error,
    num::NonZeroUsize,
    os::fd::{FromRawFd, IntoRawFd},
    sync::Arc,
};

use log::{debug, log_enabled};

use crate::prelude::*;

#[derive(Debug)]
pub struct Acceptor<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) listener: mio::net::TcpListener,
    pub(crate) callback: Arc<C>,
    pub(crate) phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Acceptor<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
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
    for Acceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Acceptor", self.con_id)
    }
}

#[derive(Debug)]
pub struct Svc<M: Messenger+'static, C: CallbackRecvSend<M>+'static, const MAX_MSG_SIZE: usize> {
    acceptor: Acceptor<M, C, MAX_MSG_SIZE>,
    clts_pool: CltsPool<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Svc<M, C, MAX_MSG_SIZE> {
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: NonZeroUsize, // TODO this arg needs better name
        name: Option<&str>,
    ) -> Result<Self, Error> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let acceptor = Acceptor {
            con_id: ConId::svc(name, addr, None),
            listener: mio::net::TcpListener::from_std(listener),
            callback,
            phantom: std::marker::PhantomData,
        };

        let clts_pool = CltsPool::<M, C, MAX_MSG_SIZE>::new(max_connections);
        Ok(Self {
            acceptor,
            clts_pool,
        })
    }

    #[inline(always)]
    pub fn len(&self) -> (usize, usize) {
        self.clts_pool.len()
    }
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
        let acceptor = PoolCltAcceptor {
            tx_recver,
            tx_sender,
            acceptor: self.acceptor,
        };
        (acceptor, svc_recver, svc_sender)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolAcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error> {
        match self.acceptor.accept_nonblocking()? {
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
    fn accept_nonblocking(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clts_pool.send_nonblocking(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.clts_pool.recv_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Svc<{}, {}>", self.acceptor, self.clts_pool)
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {
    use std::{io::ErrorKind, num::NonZeroUsize, time::Duration};

    use crate::prelude::*;
    use links_testing::unittest::setup::{
        self,
        model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
    };
    use log::{info, Level, LevelFilter};
    use rand::Rng;

    use crate::unittest::setup::framer::{
        TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE,
    };

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallback::<TestSvcMsgProtocol>::new_ref();
        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            callback.clone(),
            NonZeroUsize::new(2).unwrap(),
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);
    }

    #[test]
    fn test_svc_clt_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<TestSvcMsgProtocol>::with_level_ref(Level::Info, Level::Debug),
            NonZeroUsize::new(1).unwrap(),
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            LoggerCallback::<TestCltMsgProtocol>::with_level_ref(Level::Info, Level::Debug),
            Some("unittest"),
        )
        .unwrap();
        info!("clt: {}", clt);

        svc.pool_accept_busywait().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.len(), (1, 1));

        let mut clt_msg_inp = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
        info!("--------- PRE SPLIT ---------");
        clt.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- SVC SPLIT POOL ---------");
        let (_svc_acceptor, mut pool_recver, mut pool_sender) = svc.into_split();
        clt.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = pool_recver.recv_busywait().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- CLT SPLIT DIRECT ---------");
        let (mut clt_recv, mut clt_send) = clt.into_split();
        clt_send.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = pool_recver.recv_busywait().unwrap().unwrap();
        // info!("svc_msg_out: {:?}", svc_msg_out);
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        assert_eq!(svc_msg_out, clt_msg_inp);

        info!("--------- CLT DROP RANDOM HALF ---------");

        // drop clt_recv and ensure that clt_sender has broken pipe
        let drop_send = rand::thread_rng().gen_range(1..=2) % 2 == 0;

        if drop_send {
            info!("dropping clt_send");
            drop(clt_send);
            let opt = clt_recv.recv_nonblocking().unwrap().unwrap_completed();
            info!("clt_recv opt: {:?}", opt);
            assert_eq!(opt, None);
        } else {
            info!("dropping clt_recv");
            drop(clt_recv); // drop of recv shuts down Write half of cloned stream and hence impacts clt_send
            let err = clt_send.send_nonblocking(&mut clt_msg_inp).unwrap_err();
            info!("clt_send err: {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }

        info!("--------- SVC RECV/SEND SHOULD FAIL CLT DROPS HALF ---------");
        // recv with busywait to ensure that clt drop has delivered FIN signal and receiver does not just return WouldBlock
        let opt = pool_recver
            .recv_busywait_timeout(Duration::from_millis(100))
            .unwrap()
            .unwrap_completed();
        info!("pool_recver opt: {:?}", opt);
        assert_eq!(opt, None);
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let err = pool_sender.send_nonblocking(&mut svc_msg_inp).unwrap_err();
        info!("pool_sender err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
}
