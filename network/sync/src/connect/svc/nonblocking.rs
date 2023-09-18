use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    os::fd::{FromRawFd, IntoRawFd},
    sync::{mpsc::Sender, Arc},
};

use crate::prelude_nonblocking::*;
use links_network_core::{
    callbacks::CallbackRecvSend,
    prelude::{ConId, Messenger},
};
use log::{debug, log_enabled};

#[derive(Debug)]
pub struct Svc<M: Messenger+'static, C: CallbackRecvSend<M>+'static, const MAX_MSG_SIZE: usize> {
    pool_acceptor: PoolAcceptor<M, C, MAX_MSG_SIZE>,
    pool_recver: PoolRecver<M, C, MAX_MSG_SIZE>,
    pool_sender: PoolSender<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Svc<M, C, MAX_MSG_SIZE> {
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: usize, // TODO this arg needs better name
        name: Option<&str>,
    ) -> Result<Self, Error> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let pool = ConnectionPool::<M, C, MAX_MSG_SIZE>::new(max_connections);
        let ((tx_recver, tx_sender), (svc_recver, svc_sender)) = pool.into_split();

        let listener = mio::net::TcpListener::from_std(listener);

        let acceptor = PoolAcceptor {
            tx_recver,
            tx_sender,
            listener,
            callback,
            con_id: ConId::svc(name, addr, None),
        };

        Ok(Self {
            pool_acceptor: acceptor,
            pool_recver: svc_recver,
            pool_sender: svc_sender,
        })
    }

    pub fn pool_recv_send_len(&self) -> (usize, usize) {
        (self.pool_recver.len(), self.pool_sender.len())
    }
    pub fn into_split(
        self,
    ) -> (
        PoolAcceptor<M, C, MAX_MSG_SIZE>,
        PoolRecver<M, C, MAX_MSG_SIZE>,
        PoolSender<M, C, MAX_MSG_SIZE>,
    ) {
        (self.pool_acceptor, self.pool_recver, self.pool_sender)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolAcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error> {
        if let PoolAcceptStatus::Accepted = self.pool_acceptor.pool_accept_nonblocking()? {
            self.pool_recver.service_once_rx_queue()?;
            self.pool_sender.service_once_rx_queue()?;
            Ok(PoolAcceptStatus::Accepted)
        } else {
            Ok(PoolAcceptStatus::WouldBlock)
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.pool_acceptor.accept_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.pool_sender.send_nonblocking(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.pool_recver.recv_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        let _ = self.pool_acceptor.service_once()?;
        let _ = self.pool_recver.service_once()?;
        let _ = self.pool_sender.service_once()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Svc<{}, {}, {}>",
            self.pool_acceptor, self.pool_recver, self.pool_sender
        )
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {
    use std::io::ErrorKind;

    use crate::prelude_nonblocking::*;
    use links_testing::unittest::setup::{
        self,
        model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
    };
    use log::{info, Level, LevelFilter};

    use crate::unittest::setup::framer::{
        TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE,
    };

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallback::<TestSvcMsgProtocol>::new_ref();
        let svc =
            Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback.clone(), 2, Some("unittest"))
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
            1,
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

        svc.service_once().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.pool_recv_send_len(), (1, 1));

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
        let (clt_recv, mut clt_send) = clt.into_split();
        clt_send.send_busywait(&mut clt_msg_inp).unwrap();
        let svc_msg_out = pool_recver.recv_busywait().unwrap().unwrap();
        // info!("svc_msg_out: {:?}", svc_msg_out);
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        assert_eq!(svc_msg_out, clt_msg_inp);

        info!("--------- CLT DROP HALF ---------");
        // drop clt_recv and ensure that clt_sender has broken pipe
        drop(clt_recv);
        let err = clt_send.send_nonblocking(&mut clt_msg_inp).unwrap_err();
        info!("clt_send err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);

        info!("--------- SVC SEND SHOULD FAIL AFTER RECV is NONE ---------");
        let opt = pool_recver.recv_nonblocking().unwrap().unwrap_completed();
        info!("pool_recver opt: {:?}", opt);
        assert_eq!(opt, None);
        let err = pool_sender.send_nonblocking(&mut svc_msg_inp).unwrap_err();
        info!("pool_sender err: {}", err);
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
}
