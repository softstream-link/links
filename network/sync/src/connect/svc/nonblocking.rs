use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    os::fd::{FromRawFd, IntoRawFd},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use crate::prelude_nonblocking::*;
use links_network_core::{
    callbacks::CallbackSendRecv,
    prelude::{CallbackRecv, CallbackSend, ConId, Messenger},
};
use log::{debug, log_enabled, warn};
use slab::Slab;

#[derive(Debug)]
pub struct SvcRecver<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
    svc_recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> SvcRecver<M, C, MAX_MSG_SIZE> {
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Error> {
        match self.rx_recver.try_recv() {
            Ok(recver) => {
                if self.svc_recvers.len() < self.svc_recvers.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding recver: {} to {}", recver, self);
                    }
                    self.svc_recvers.insert(recver);
                } else {
                    warn!("Dropping recver: {}, {} at capacity", recver, self,);
                }
                Ok(())
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(()),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
    #[inline]
    fn service_once_recvers(&mut self) -> Result<(), Error> {
        let (mut dead_key, mut dead_connection) = (0, false);
        for (key, clt) in self.svc_recvers.iter_mut() {
            match clt.service_once() {
                Ok(ServiceLoopStatus::Continue) => {}
                Ok(ServiceLoopStatus::Stop) => {
                    dead_connection = true;
                    dead_key = key;
                    break;
                }
                Err(e) => {
                    warn!(
                        "recver #{} is dead {} and will be dropped.  error: {}",
                        key, clt, e
                    );
                    dead_connection = true;
                    dead_key = key;
                    break;
                }
            };
        }
        // TODO fix this so that we only iterate each clt once
        if dead_connection {
            self.svc_recvers.remove(dead_key);
            self.service_once_recvers()?;
        }
        Ok(())
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for SvcRecver<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        self.service_once_recvers()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for SvcRecver<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SvcRecver<{} of {} [{}]>",
            self.svc_recvers.len(),
            self.svc_recvers.capacity(),
            self.svc_recvers
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

#[derive(Debug)]
pub struct SvcSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
    svc_senders: Slab<CltSender<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SvcSender<M, C, MAX_MSG_SIZE> {
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Error> {
        match self.rx_sender.try_recv() {
            Ok(sender) => {
                if self.svc_senders.len() < self.svc_senders.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding sender: {} to {}", sender, self);
                    }
                    self.svc_senders.insert(sender);
                } else {
                    warn!("Dropping sender: {}, {} at capacity", sender, self);
                }
                Ok(())
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(()),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for SvcSender<M, C, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for SvcSender<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SvcSender<{} of {} [{}]>",
            self.svc_senders.len(),
            self.svc_senders.capacity(),
            self.svc_senders
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}
#[derive(Debug)]
pub struct SvcAcceptor<
    M: Messenger+'static,
    C: CallbackSendRecv<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    listener: mio::net::TcpListener,
    callback: Arc<C>,
    con_id: ConId,
}
impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // TODO add rate limiter
                let stream = unsafe { std::net::TcpStream::from_raw_fd(stream.into_raw_fd()) };
                let mut con_id = self.con_id.clone();
                con_id.set_peer(addr);
                if log_enabled!(log::Level::Debug) {
                    debug!("{} Accepted", con_id);
                }
                let clt = Clt::<_, _, MAX_MSG_SIZE>::from_stream(
                    stream,
                    con_id.clone(),
                    self.callback.clone(),
                );
                Ok(Some(clt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        if let Some(clt) = self.accept_nonblocking()? {
            let (recver, sender) = clt.into_split();
            if let Err(e) = self.tx_recver.send(recver) {
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
            if let Err(e) = self.tx_sender.send(sender) {
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
        }
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> Display
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} SvcAcceptor", self.con_id)
    }
}

#[derive(Debug)]
pub struct Svc<M: Messenger+'static, C: CallbackSendRecv<M>+'static, const MAX_MSG_SIZE: usize> {
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
    recver: SvcRecver<M, C, MAX_MSG_SIZE>,
    sender: SvcSender<M, C, MAX_MSG_SIZE>,
}

impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> Svc<M, C, MAX_MSG_SIZE> {
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: usize, // TODO this arg needs better name
        name: Option<&str>,
    ) -> Result<Self, Error> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        let listener = mio::net::TcpListener::from_std(listener);

        let acceptor = SvcAcceptor {
            tx_recver,
            tx_sender,
            listener,
            callback,
            con_id: ConId::svc(name, addr, None),
        };
        let svc_recver = SvcRecver {
            rx_recver,
            svc_recvers: Slab::with_capacity(max_connections),
        };
        let svc_sender = SvcSender {
            rx_sender,
            svc_senders: Slab::with_capacity(max_connections),
        };
        Ok(Self {
            acceptor,
            recver: svc_recver,
            sender: svc_sender,
        })
    }

    pub fn clients_len(&self) -> (usize, usize) {
        (self.recver.svc_recvers.len(), self.sender.svc_senders.len())
    }
    pub fn split_into(
        self,
    ) -> (
        SvcAcceptor<M, C, MAX_MSG_SIZE>,
        SvcRecver<M, C, MAX_MSG_SIZE>,
        SvcSender<M, C, MAX_MSG_SIZE>,
    ) {
        (self.acceptor, self.recver, self.sender)
    }
}
impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept_nonblocking()
    }
}

impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        let _ = self.acceptor.service_once()?;
        let _ = self.recver.service_once()?;
        let _ = self.sender.service_once()?;
        Ok(ServiceLoopStatus::Continue)
    }
}

impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} Svc<{}, {}, {}>",
            self.acceptor.con_id, self.acceptor, self.recver, self.sender
        )
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {
    use std::io::ErrorKind;

    use crate::prelude_nonblocking::*;
    use links_network_core::callbacks::{
        devnull_new::DevNullCallbackNew, logger_new::LoggerCallbackNew,
    };
    use links_testing::unittest::setup::model::TestSvcMsg;
    use links_testing::unittest::setup::{self, model::TestSvcMsgDebug};
    use log::{info, warn, LevelFilter};

    use crate::{
        connect::clt::nonblocking::Clt,
        unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE},
    };

    use super::Svc;

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallbackNew::<TestSvcMsgProtocol>::new_ref();
        let svc =
            Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback.clone(), 2, Some("unittest"))
                .unwrap();
        info!("svc: {}", svc);
    }
    #[test]
    fn test_svc_clt_connected_svc_max_connections() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = 2;

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            DevNullCallbackNew::<TestSvcMsgProtocol>::new_ref(),
            max_connections,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clts = vec![];
        for i in 0..max_connections * 2 {
            let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
                addr,
                std::time::Duration::from_millis(50),
                std::time::Duration::from_millis(10),
                DevNullCallbackNew::<TestCltMsgProtocol>::new_ref(),
                Some("unittest"),
            )
            .unwrap();
            info!("#{}, clt: {}", i, clt);
            clts.push(clt);
            svc.service_once().unwrap();
        }

        let (recv_count, send_count) = svc.clients_len();
        info!(
            "svc: recv_count: {}, send_count: {}",
            recv_count, send_count
        );
        assert_eq!(recv_count, max_connections);
        assert_eq!(send_count, max_connections);
    }
    #[test]
    fn test_svc_clt_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();

        let callback = LoggerCallbackNew::<TestSvcMsgProtocol>::new_ref();
        let mut svc =
            Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, callback.clone(), 1, Some("unittest"))
                .unwrap();
        info!("svc: {}", svc);

        let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            std::time::Duration::from_millis(50),
            std::time::Duration::from_millis(10),
            callback.clone(),
            Some("unittest"),
        )
        .unwrap();
        info!("clt: {}", clt);

        svc.service_once().unwrap();
        info!("svc: {}", svc);
        assert_eq!(svc.clients_len(), (1, 1));

        let (clt_recv, mut clt_send) = clt.into_split();

        // drop clt_recv and ensure that clt_sender has broken pipe
        drop(clt_recv);

        let mut clt_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Client Msg"));
        let err = clt_send.send_busywait(&mut clt_msg).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        warn!("{}", err);
    }
}
