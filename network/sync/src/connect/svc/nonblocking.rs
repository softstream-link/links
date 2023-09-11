use std::{
    error::Error,
    fmt::Display,
    os::fd::{FromRawFd, IntoRawFd},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use crate::{
    core::nonblocking::{AcceptCltBusyWait, AcceptCltNonBlocking},
    prelude_nonblocking::*,
};
use links_network_core::{
    callbacks::CallbackSendRecvNew,
    prelude::{CallbackRecv, CallbackSend, ConId, MessengerNew},
};
use log::{debug, info, log_enabled, warn};
use slab::Slab;

use crate::connect::clt::nonblocking::{Clt, CltRecver, CltSender};

#[derive(Debug)]
pub struct SvcRecver<M: MessengerNew+'static, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, CRecv, MAX_MSG_SIZE>>,
    svc_recvers: Slab<CltRecver<M, CRecv, MAX_MSG_SIZE>>,
}
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize>
    SvcRecver<M, CRecv, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Box<dyn Error>> {
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
            Err(e) => Err(e.into()),
        }
    }
    #[inline]
    fn service_once_recvers(&mut self) -> Result<(), Box<dyn Error>> {
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
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for SvcRecver<M, CRecv, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        self.service_once_rx_queue()?;
        self.service_once_recvers()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for SvcRecver<M, CRecv, MAX_MSG_SIZE>
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
pub struct SvcSender<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, CSend, MAX_MSG_SIZE>>,
    svc_senders: Slab<CltSender<M, CSend, MAX_MSG_SIZE>>,
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize>
    SvcSender<M, CSend, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Box<dyn Error>> {
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
            Err(e) => Err(e.into()),
        }
    }
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for SvcSender<M, CSend, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        self.service_once_rx_queue()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for SvcSender<M, CSend, MAX_MSG_SIZE>
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
    M: MessengerNew+'static,
    C: CallbackSendRecvNew<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    listener: mio::net::TcpListener,
    callback: Arc<C>,
    con_id: ConId,
}

impl<M: MessengerNew+'static, C: CallbackSendRecvNew<M>+'static, const MAX_MSG_SIZE: usize>
    AcceptCltBusyWait<M, C, MAX_MSG_SIZE> for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept_busywait(
        &self,
        timeout: Duration,
    ) -> Result<Clt<M, C, MAX_MSG_SIZE>, Box<dyn Error>> {
        let now = Instant::now();
        loop {
            let clt = self.accept_nonblocking()?;
            match clt {
                Some(clt) => return Ok(clt),
                None => {
                    if now.elapsed() > timeout {
                        return Err(format!("accept timeout: {:?}", timeout).into());
                    }
                    continue;
                }
            }
        }
    }
}
impl<M: MessengerNew+'static, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Box<dyn Error>> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // TODO add rate limiter
                let stream = unsafe { std::net::TcpStream::from_raw_fd(stream.into_raw_fd()) };
                let mut con_id = self.con_id.clone();
                con_id.set_peer(addr);
                info!("{} Accepted", con_id);
                let clt = Clt::<_, _, MAX_MSG_SIZE>::from_stream(
                    stream,
                    con_id.clone(),
                    self.callback.clone(),
                );
                Ok(Some(clt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

impl<M: MessengerNew+'static, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize>
    NonBlockingServiceLoop for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        if let Some(clt) = self.accept_nonblocking()? {
            let (recver, sender) = clt.into_split();
            self.tx_recver.send(recver)?;
            self.tx_sender.send(sender)?;
        }
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> Display
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} SvcAcceptor", self.con_id)
    }
}

#[derive(Debug)]
pub struct Svc<
    M: MessengerNew+'static,
    C: CallbackSendRecvNew<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
    svc_recver: SvcRecver<M, C, MAX_MSG_SIZE>,
    svc_sender: SvcSender<M, C, MAX_MSG_SIZE>,
}

impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize>
    Svc<M, C, MAX_MSG_SIZE>
{
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: usize,
        name: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
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
            svc_recver,
            svc_sender,
        })
    }

    pub fn clients_len(&self) -> (usize, usize) {
        (
            self.svc_recver.svc_recvers.len(),
            self.svc_sender.svc_senders.len(),
        )
    }
    pub fn split_into(
        self,
    ) -> (
        SvcAcceptor<M, C, MAX_MSG_SIZE>,
        SvcRecver<M, C, MAX_MSG_SIZE>,
        SvcSender<M, C, MAX_MSG_SIZE>,
    ) {
        (self.acceptor, self.svc_recver, self.svc_sender)
    }
}
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize>
    AcceptCltBusyWait<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn accept_busywait(
        &self,
        timeout: Duration,
    ) -> Result<Clt<M, C, MAX_MSG_SIZE>, Box<dyn Error>> {
        self.acceptor.accept_busywait(timeout)
    }
}
impl<M: MessengerNew+'static, C: CallbackSendRecvNew<M>+'static, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for Svc<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Box<dyn Error>> {
        self.acceptor.accept_nonblocking()
    }
}

impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        let _ = self.acceptor.service_once()?;
        let _ = self.svc_recver.service_once()?;
        let _ = self.svc_sender.service_once()?;
        Ok(ServiceLoopStatus::Continue)
    }
}

impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} Svc<{}, {}, {}>",
            self.acceptor.con_id, self.acceptor, self.svc_recver, self.svc_sender
        )
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {
    use crate::prelude_nonblocking::*;
    use links_network_core::callbacks::{
        devnull_new::DevNullCallbackNew, logger_new::LoggerCallbackNew,
    };
    use links_testing::unittest::setup;
    use log::{info, LevelFilter};

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
        setup::log::configure();
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
    }
}
