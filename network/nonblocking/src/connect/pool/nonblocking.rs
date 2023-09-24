use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    os::fd::{FromRawFd, IntoRawFd},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use crate::{
    core::iter::CycleRange,
    prelude_nonblocking::{
        AcceptCltNonBlocking, AcceptStatus, CallbackRecv, CallbackRecvSend, CallbackSend, Clt,
        CltRecver, CltSender, ConId, Messenger, NonBlockingServiceLoop, PoolAcceptCltNonBlocking,
        PoolAcceptStatus, RecvMsgNonBlocking, RecvStatus, SendMsgNonBlocking, SendStatus,
        ServiceLoopStatus,
    },
};
use log::{debug, info, log_enabled, warn};
use slab::Slab;

pub struct ConnectionPool<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    recver_pool: PoolRecver<M, C, MAX_MSG_SIZE>,
    sender_pool: PoolSender<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    ConnectionPool<M, C, MAX_MSG_SIZE>
{
    pub fn new(max_connections: usize) -> Self {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        Self {
            tx_recver,
            tx_sender,
            recver_pool: PoolRecver::new(rx_recver, max_connections),
            sender_pool: PoolSender::new(rx_sender, max_connections),
        }
    }
    pub fn into_split(
        self,
    ) -> (
        (
            Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
            Sender<CltSender<M, C, MAX_MSG_SIZE>>,
        ),
        (
            PoolRecver<M, C, MAX_MSG_SIZE>,
            PoolSender<M, C, MAX_MSG_SIZE>,
        ),
    ) {
        (
            (self.tx_recver, self.tx_sender),
            (self.recver_pool, self.sender_pool),
        )
    }
}

#[derive(Debug)]
pub struct PoolRecver<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
    recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> PoolRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>, max_connections: usize) -> Self {
        Self {
            rx_recver,
            recvers: Slab::with_capacity(max_connections),
            slab_keys: CycleRange::new(0..max_connections),
        }
    }
    pub fn len(&self) -> usize {
        self.recvers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recvers.is_empty()
    }
    /// returns true if a recver was added
    #[inline]
    pub(crate) fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
        match self.rx_recver.try_recv() {
            Ok(recver) => {
                if self.recvers.len() < self.recvers.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding recver: {} to {}", recver, self);
                    }
                    self.recvers.insert(recver);
                } else {
                    warn!("Dropping recver: {}, {} at capacity", recver, self);
                }
                Ok(true)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(false),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
    #[inline]
    fn next_key_recver_mut(&mut self) -> Option<(usize, &mut CltRecver<M, C, MAX_MSG_SIZE>)> {
        for _ in 0..self.recvers.len() {
            let key = self.slab_keys.next();
            if self.recvers.contains(key) {
                return Some((key, &mut self.recvers[key]));
            }
        }
        None
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for PoolRecver<M, C, MAX_MSG_SIZE>
{
    /// Will round robin available recvers. If the recver connection is dead it will be removed and next recver will be tried.
    /// If all recvers are exhausted the rx_queue will be checked to see if a new recver is available.
    #[inline]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        use log::Level::{Info, Warn};
        use RecvStatus::{Completed, WouldBlock};
        match self.next_key_recver_mut() {
            Some((key, clt)) => match clt.recv_nonblocking() {
                Ok(Completed(Some(msg))) => return Ok(Completed(Some(msg))),
                Ok(WouldBlock) => return Ok(WouldBlock),
                Ok(Completed(None)) => {
                    let recver = self.recvers.remove(key);
                    if log_enabled!(Info) {
                        info!(
                                "Connection reset by peer, clean. key: #{}, recver: {} and will be dropped, recvers: {}",
                                key, recver, self
                            );
                    }
                    return Ok(Completed(None));
                }
                Err(e) => {
                    let recver = self.recvers.remove(key);
                    if log_enabled!(Warn) {
                        warn!(
                                "Connection failed, {}. key: #{}, recver: {} and will be dropped.  recvers: {}",
                                e, key, recver, self
                            );
                    }
                    return Err(e);
                }
            },
            None => {
                // no recivers available try processing rx_queue
                if self.service_once_rx_queue()? {
                    self.recv_nonblocking()
                } else {
                    Err(Error::new(
                        ErrorKind::NotConnected,
                        "Not Connected, 0 recvers available in the pool",
                    ))
                }
            }
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for PoolRecver<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        let _ = self.recv_nonblocking()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for PoolRecver<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PoolRecver<len: {} of cap: {} [{}]>",
            self.recvers.len(),
            self.recvers.capacity(),
            self.recvers
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

#[derive(Debug)]
pub struct PoolSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
    senders: Slab<CltSender<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> PoolSender<M, C, MAX_MSG_SIZE> {
    pub fn new(rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>, max_connections: usize) -> Self {
        Self {
            rx_sender,
            senders: Slab::with_capacity(max_connections),
            slab_keys: CycleRange::new(0..max_connections),
        }
    }
    pub fn len(&self) -> usize {
        self.senders.len()
    }
    pub fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }
    #[inline]
    pub(crate) fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
        match self.rx_sender.try_recv() {
            Ok(sender) => {
                if self.senders.len() < self.senders.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding sender: {} to {}", sender, self);
                    }
                    self.senders.insert(sender);
                } else {
                    warn!("Dropping sender: {}, {} at capacity", sender, self);
                }
                Ok(true)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(false),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }

    #[inline]
    fn next_key_sender_mut(&mut self) -> Option<(usize, &mut CltSender<M, C, MAX_MSG_SIZE>)> {
        // TODO can this be optimized
        for _ in 0..self.senders.len() {
            let key = self.slab_keys.next();
            if self.senders.contains(key) {
                return Some((key, &mut self.senders[key]));
            }
        }
        None
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for PoolSender<M, C, MAX_MSG_SIZE>
{
    /// Each call to this method will use the next available sender by using round round on each subsequent call.
    /// if the are no senders available the rx_queue will be checked once and if a new sender is available it will be used.
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        match self.next_key_sender_mut() {
            Some((key, clt)) => {
                match clt.send_nonblocking(msg) {
                    Ok(SendStatus::Completed) => return Ok(SendStatus::Completed),
                    Ok(SendStatus::WouldBlock) => return Ok(SendStatus::WouldBlock),
                    Err(e) => {
                        let msg = format!(
                            "sender #{} is dead {} and will be dropped.  error: ({})",
                            key, self, e
                        );
                        self.senders.remove(key);
                        return Err(Error::new(e.kind(), msg));
                    }
                };
            }
            None => {
                // no senders available try processing rx_queue
                if self.service_once_rx_queue()? {
                    self.send_nonblocking(msg)
                } else {
                    Err(Error::new(
                        ErrorKind::NotConnected,
                        "Not Connected, 0 senders available in the pool",
                    ))
                }
            }
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for PoolSender<M, C, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for PoolSender<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PoolSender<len: {} of cap: {} [{}]>",
            self.senders.len(),
            self.senders.capacity(),
            self.senders
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

#[derive(Debug)]
pub struct PoolAcceptor<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    pub(crate) tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    pub(crate) tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    pub(crate) listener: mio::net::TcpListener,
    pub(crate) callback: Arc<C>,
    pub(crate) con_id: ConId,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolAcceptor<M, C, MAX_MSG_SIZE>
{
    //TODO add new method and use it in svc/nonblocking.rs
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolAcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for PoolAcceptor<M, C, MAX_MSG_SIZE>
{
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error> {
        if let AcceptStatus::Accepted(clt) = self.accept_nonblocking()? {
            let (recver, sender) = clt.into_split();
            if let Err(e) = self.tx_recver.send(recver) {
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
            if let Err(e) = self.tx_sender.send(sender) {
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
        }
        Ok(PoolAcceptStatus::Accepted)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for PoolAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept_nonblocking(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
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
                Ok(AcceptStatus::Accepted(clt))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(AcceptStatus::WouldBlock),
            Err(e) => Err(e),
        }
    }
}

impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for PoolAcceptor<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.pool_accept_nonblocking()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for PoolAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} PoolAcceptor", self.con_id)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        prelude_nonblocking::*,
        unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE},
    };
    use links_network_core::prelude::DevNullCallback;
    use links_testing::unittest::setup;
    use log::{info, LevelFilter};

    #[test]
    fn test_svc_clt_connected_svc_max_connections() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = 2;

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            DevNullCallback::<TestSvcMsgProtocol>::new_ref(),
            max_connections,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clts = vec![];
        for i in 0..max_connections * 2 {
            let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
                addr,
                setup::net::default_connect_timeout(),
                setup::net::default_connect_retry_after(),
                DevNullCallback::<TestCltMsgProtocol>::new_ref(),
                Some("unittest"),
            )
            .unwrap();
            info!("#{}, clt: {}", i, clt);
            clts.push(clt);
            svc.service_once().unwrap();
        }

        let (recv_count, send_count) = svc.pool_recv_send_len();
        info!(
            "svc: recv_count: {}, send_count: {}",
            recv_count, send_count
        );
        assert_eq!(recv_count, max_connections);
        assert_eq!(send_count, max_connections);
    }
}
