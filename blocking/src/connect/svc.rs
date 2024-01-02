use super::pool::PoolCltAcceptor;
use crate::prelude::{AcceptClt, CallbackRecvSend, Clt, CltRecversPool, CltSendersPool, CltsPool, ConId, Messenger, PoolAcceptClt, RecvMsg, SendMsg};
use links_core::asserted_short_name;
use log::{debug, log_enabled};
use std::{fmt::Display, io::Error, net::TcpListener, num::NonZeroUsize, sync::Arc};

#[derive(Debug)]
pub struct SvcAcceptor<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    callback: Arc<C>,
    listener: TcpListener,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SvcAcceptor<M, C, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, listener: TcpListener, callback: Arc<C>) -> Self {
        Self {
            con_id,
            callback,
            listener,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE> for SvcAcceptor<M, C, MAX_MSG_SIZE> {
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // TODO add rate limiter
                let mut con_id = self.con_id.clone();
                con_id.set_peer(addr);
                if log_enabled!(log::Level::Debug) {
                    debug!("{} Accepted", con_id);
                }
                let clt = Clt::<_, _, MAX_MSG_SIZE>::from_stream(stream, con_id.clone(), self.callback.clone());
                Ok(clt)
            }
            Err(e) => Err(e),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display for SvcAcceptor<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}>", asserted_short_name!("SvcAcceptor", Self), self.con_id)
    }
}

pub struct Svc<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
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

        let acceptor = SvcAcceptor {
            con_id: ConId::svc(name, addr, None),
            callback,
            listener,
            phantom: std::marker::PhantomData,
        };
        let clts_pool = CltsPool::with_capacity(max_connections);

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
    pub fn pool(&self) -> &CltsPool<M, C, MAX_MSG_SIZE> {
        &self.clts_pool
    }
    #[inline(always)]
    pub fn into_split(self) -> (PoolCltAcceptor<M, C, MAX_MSG_SIZE>, CltRecversPool<M, C, MAX_MSG_SIZE>, CltSendersPool<M, C, MAX_MSG_SIZE>) {
        let ((tx_recver, tx_sender), (svc_recv, svc_send)) = self.clts_pool.into_split();
        let pool_acceptor = PoolCltAcceptor::new(tx_recver, tx_sender, self.acceptor);
        (pool_acceptor, svc_recv, svc_send)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE> {
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        self.acceptor.accept()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsg<M> for Svc<M, C, MAX_MSG_SIZE> {
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        self.clts_pool.send(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsg<M> for Svc<M, C, MAX_MSG_SIZE> {
    fn recv(&mut self) -> Result<Option<<M as Messenger>::RecvT>, Error> {
        self.clts_pool.recv()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> PoolAcceptClt<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE> {
    fn pool_accept(&mut self) -> Result<(), Error> {
        match self.acceptor.accept() {
            Ok(clt) => self.clts_pool.add(clt),
            Err(e) => Err(e),
        }
    }
}

impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display for Svc<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}, {}>", asserted_short_name!("Svc", Self), self.acceptor, self.clts_pool,)
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {

    use crate::prelude::*;
    use links_core::{
        prelude::{DevNullCallback, LoggerCallback},
        unittest::setup::{
            self,
            framer::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
            messenger::CltTestMessenger,
            model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg, SvcTestMsgDebug},
        },
    };
    use log::{info, LevelFilter};
    use rand::Rng;
    use std::num::NonZeroUsize;

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, DevNullCallback::<SvcTestMessenger>::new_ref(), NonZeroUsize::new(2).unwrap(), Some("unittest")).unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.pool().len(), 0);
    }

    #[test]
    fn test_svc_clt_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, LoggerCallback::<SvcTestMessenger>::new_ref(), NonZeroUsize::new(2).unwrap(), Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            LoggerCallback::<CltTestMessenger>::new_ref(),
            Some("unittest"),
        )
        .unwrap();
        info!("clt: {}", clt);

        svc.pool_accept().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.len(), 1);

        let mut clt_msg_inp = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        let mut svc_msg_inp = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));

        info!("--------- PRE SPLIT ---------");
        clt.send(&mut clt_msg_inp).unwrap();
        let svc_msg_out = svc.recv().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- SVC SPLIT POOL ---------");
        let (_svc_acceptor, mut pool_recver, mut pool_sender) = svc.into_split();
        clt.send(&mut clt_msg_inp).unwrap();
        let svc_msg_out = pool_recver.recv().unwrap().unwrap();
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        // info!("svc_msg_out: {:?}", svc_msg_out);
        assert_eq!(clt_msg_inp, svc_msg_out);

        info!("--------- CLT SPLIT DIRECT ---------");
        let (mut clt_recv, mut clt_send) = clt.into_split();
        clt_send.send(&mut clt_msg_inp).unwrap();
        let svc_msg_out = pool_recver.recv().unwrap().unwrap();
        // info!("svc_msg_out: {:?}", svc_msg_out);
        // info!("clt_msg_inp: {:?}", clt_msg_inp);
        assert_eq!(svc_msg_out, clt_msg_inp);

        info!("--------- CLT DROP RANDOM HALF ---------");
        // drop clt_recv and ensure that clt_sender has broken pipe
        let drop_send = rand::thread_rng().gen_range(1..=2) % 2 == 0;

        if drop_send {
            info!("dropping clt_send");
            drop(clt_send);
            let opt = clt_recv.recv().unwrap();
            info!("clt_recv opt: {:?}", opt);
            assert_eq!(opt, None);
        } else {
            info!("dropping clt_recv");
            drop(clt_recv); // drop of recv shuts down Write half of cloned stream and hence impacts clt_send
            let err = clt_send.send(&mut clt_msg_inp).unwrap_err();
            info!("clt_send err: {}", err);
            assert_error_kind_on_target_family!(err, std::io::ErrorKind::BrokenPipe);
        }

        info!("--------- SVC RECV/SEND SHOULD FAIL CLT DROPS HALF ---------");
        // recv with busywait to ensure that clt drop has delivered FIN signal and receiver does not just return WouldBlock
        let opt = pool_recver.recv().unwrap();
        info!("pool_recver opt: {:?}", opt);
        assert_eq!(opt, None);
        // because pool_recver will get None it will understand that the client socket is closed and hence will shutdown the write
        // direction which in turn will force send to fail with ErrorKind::BrokenPipe
        let err = pool_sender.send(&mut svc_msg_inp).unwrap_err();
        info!("pool_sender err: {}", err);
        assert_error_kind_on_target_family!(err, std::io::ErrorKind::BrokenPipe);
    }
}
