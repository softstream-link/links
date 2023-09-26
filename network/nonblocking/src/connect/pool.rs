use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    num::NonZeroUsize,
    os::fd::{FromRawFd, IntoRawFd},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use crate::{
    core::iter::CycleRange,
    prelude::{
        AcceptCltNonBlocking, AcceptStatus, CallbackRecv, CallbackRecvSend, CallbackSend, Clt,
        CltRecver, CltSender, ConId, Messenger, NonBlockingServiceLoop, PoolAcceptCltNonBlocking,
        PoolAcceptStatus, RecvMsgNonBlocking, RecvStatus, SendMsgNonBlocking, SendStatus,
        ServiceLoopStatus,
    },
};
use log::{debug, info, log_enabled, warn};
use slab::Slab;

/// An abstraction layer for creating a [RecversPool] and [SendersPool] with respective [std::sync::mpsc::Sender]
/// channel for each pool so that the user of the pool can run in a thread different from a thread that is populating the pool.
///
/// # Example
/// ```
/// use links_network_nonblocking::prelude::*;
/// use links_network_nonblocking::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE};
/// use links_testing::unittest::setup::model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug};
/// use std::time::Duration;
///
///
/// let mut pool = ConnectionPool::default();
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     "127.0.0.1:8080",
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::<TestCltMsgProtocol>::default().into(),
///     Some("unittest"),
/// );
///
/// if res.is_ok() {
///     pool.add(res.unwrap());
///
///     let mut clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
///     // Not Split for use in single thread
///     pool.send_busywait(&mut clt_msg).unwrap();
///     let svc_msg: TestSvcMsg = pool.recv_busywait().unwrap().unwrap();
///
///     // Split for use different threads
///     let ((tx_recver, tx_sender), (mut recvers, mut senders)) = pool.into_split();
///     senders.send_busywait(&mut clt_msg).unwrap();
///     let svc_msg: TestSvcMsg = recvers.recv_busywait().unwrap().unwrap();
/// }
/// ```
pub struct ConnectionPool<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    recver_pool: RecversPool<M, C, MAX_MSG_SIZE>,
    sender_pool: SendersPool<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    ConnectionPool<M, C, MAX_MSG_SIZE>
{
    /// Creates a new [ConnectionPool]
    /// # Arguments
    ///  * max_connections - the maximum number of connections that can be added to the pool.
    ///
    /// # Important
    ///  * The channel will continue to accept new connections even if the pool is at capacity. However, `once and only once`
    /// the [RecversPool] or [SendersPool] services its respective channel the connection will be dropped respective pool is at capacity.
    pub fn new(max_connections: NonZeroUsize) -> Self {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        Self {
            tx_recver,
            tx_sender,
            recver_pool: RecversPool::new(rx_recver, max_connections),
            sender_pool: SendersPool::new(rx_sender, max_connections),
        }
    }
    /// Returns a tuple representing len of [RecversPool] and [SendersPool] respectively
    pub fn len(&self) -> (usize, usize) {
        (self.recver_pool.len(), self.sender_pool.len())
    }
    pub fn has_capacity(&self) -> bool {
        self.recver_pool.has_capacity() && self.sender_pool.has_capacity()
    }
    /// Adds a [Clt] to the pool
    pub fn add(&mut self, clt: Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
        if !self.recver_pool.has_capacity() {
            return Err(Error::new(
                ErrorKind::Other,
                "ConnectionPool recver_pool at capacity",
            ));
        }
        if !self.sender_pool.has_capacity() {
            return Err(Error::new(
                ErrorKind::Other,
                "ConnectionPool sender_pool at capacity",
            ));
        }

        let (recver, sender) = clt.into_split();
        if let Err(e) = self.tx_recver.send(recver) {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        if let Err(e) = self.tx_sender.send(sender) {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        self.recver_pool.service_once_rx_queue()?;
        self.sender_pool.service_once_rx_queue()?;
        Ok(())
    }
    /// Splits [ConnectionPool] into a a pair of channel transmitters and their respective pools
    pub fn into_split(self) -> SplitConnectionPool<M, C, MAX_MSG_SIZE> {
        (
            (self.tx_recver, self.tx_sender),
            (self.recver_pool, self.sender_pool),
        )
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Default
    for ConnectionPool<M, C, MAX_MSG_SIZE>
{
    /// Creates a new [ConnectionPool] with a max_connections of 1
    fn default() -> Self {
        Self::new(NonZeroUsize::new(1).unwrap())
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for ConnectionPool<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.sender_pool.send_nonblocking(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for ConnectionPool<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.recver_pool.recv_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for ConnectionPool<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.sender_pool.service_once()?;
        self.recver_pool.service_once()?;
        Ok(ServiceLoopStatus::Continue)
    }
}

pub type SplitConnectionPool<M, C, const MAX_MSG_SIZE: usize> = (
    (
        Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
        Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    ),
    (
        RecversPool<M, C, MAX_MSG_SIZE>,
        SendersPool<M, C, MAX_MSG_SIZE>,
    ),
);

/// An abstraction layer handling a pool of round robin [CltRecver]'s with respective [std::sync::mpsc::Receiver] channel
/// that is inspected in order to add additional [CltRecver]'s to the pool.
#[derive(Debug)]
pub struct RecversPool<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
    recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecversPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new instance of [RecversPool]
    pub fn new(
        rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
        max_connections: NonZeroUsize,
    ) -> Self {
        Self {
            rx_recver,
            recvers: Slab::with_capacity(max_connections.get()),
            slab_keys: CycleRange::new(0..max_connections.get()),
        }
    }

    pub fn len(&self) -> usize {
        self.recvers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.recvers.is_empty()
    }
    pub fn clear(&mut self) {
        self.recvers.clear();
    }
    #[inline]
    pub fn has_capacity(&self) -> bool {
        self.recvers.len() < self.recvers.capacity()
    }
    /// returns true if a recver was added
    #[inline]
    pub(crate) fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
        match self.rx_recver.try_recv() {
            Ok(recver) => {
                if self.has_capacity() {
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
    fn next(&mut self) -> Option<(usize, &mut CltRecver<M, C, MAX_MSG_SIZE>)> {
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
    for RecversPool<M, C, MAX_MSG_SIZE>
{
    /// Will round robin available recvers. If the recver connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    ///
    /// # Important
    ///
    /// This method will not check internal `rx_recver` channel for new recvers unless the pool is fully exhausted and empty.
    /// In the event there are no receivers in the channel or the pool the method will return an [Error] where `e.kind() == ErrorKind::NotConnected`
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        use log::Level::{Info, Warn};
        use RecvStatus::{Completed, WouldBlock};
        match self.next() {
            Some((key, clt)) => match clt.recv_nonblocking() {
                Ok(Completed(Some(msg))) => Ok(Completed(Some(msg))),
                Ok(WouldBlock) => Ok(WouldBlock),
                Ok(Completed(None)) => {
                    let recver = self.recvers.remove(key);
                    if log_enabled!(Info) {
                        info!(
                                "Connection reset by peer, clean. key: #{}, recver: {} and will be dropped, recvers: {}",
                                key, recver, self
                            );
                    }
                    Ok(Completed(None))
                }
                Err(e) => {
                    let recver = self.recvers.remove(key);
                    if log_enabled!(Warn) {
                        warn!(
                                "Connection failed, {}. key: #{}, recver: {} and will be dropped.  recvers: {}",
                                e, key, recver, self
                            );
                    }
                    Err(e)
                }
            },
            None => {
                // no receivers available try processing rx_queue
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
    for RecversPool<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        let _ = self.recv_nonblocking()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for RecversPool<M, C, MAX_MSG_SIZE>
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

/// An abstraction layer handling a pool of round robin [SendersPool]'s with respective [std::sync::mpsc::Receiver] channel
/// that is inspected in order to add additional [SendersPool]'s to the pool.
#[derive(Debug)]
pub struct SendersPool<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
    senders: Slab<CltSender<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendersPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new instance of [SendersPool]
    pub fn new(
        rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
        max_connections: NonZeroUsize,
    ) -> Self {
        Self {
            rx_sender,
            senders: Slab::with_capacity(max_connections.get()),
            slab_keys: CycleRange::new(0..max_connections.get()),
        }
    }
    pub fn len(&self) -> usize {
        self.senders.len()
    }
    pub fn is_empty(&self) -> bool {
        self.senders.is_empty()
    }
    pub fn clear(&mut self) {
        self.senders.clear();
    }
    #[inline]
    pub fn has_capacity(&self) -> bool {
        self.senders.len() < self.senders.capacity()
    }
    #[inline]
    pub(crate) fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
        match self.rx_sender.try_recv() {
            Ok(sender) => {
                if self.has_capacity() {
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
    fn next(&mut self) -> Option<(usize, &mut CltSender<M, C, MAX_MSG_SIZE>)> {
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
    for SendersPool<M, C, MAX_MSG_SIZE>
{
    /// Will round robin available senders. If the sender connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    ///
    /// # Important
    ///
    /// This method will not check internal `rx_sender` channel for new senders unless the pool is fully exhausted and empty.
    /// In the event there are no receivers in the channel or the pool the method will return an [Error] where `e.kind() == ErrorKind::NotConnected`
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        match self.next() {
            Some((key, clt)) => match clt.send_nonblocking(msg) {
                Ok(s) => Ok(s),
                Err(e) => {
                    let msg = format!(
                        "sender #{} is dead {} and will be dropped.  error: ({})",
                        key, self, e
                    );
                    self.senders.remove(key);
                    Err(Error::new(e.kind(), msg))
                }
            },
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
    for SendersPool<M, C, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for SendersPool<M, C, MAX_MSG_SIZE>
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

/// An abstraction layer contains contains a [mio::net::TcpListener] and methods for accepting new connections.
/// Connections can be accepted and returned to the caller directly or added to the sender and recver pools.
///
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
    use std::num::NonZeroUsize;

    use crate::{
        prelude::*,
        unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE},
    };
    use links_network_core::prelude::DevNullCallback;
    use links_testing::unittest::setup;
    use log::{info, LevelFilter};

    #[test]
    fn test_svcpool_cltpool_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = NonZeroUsize::new(2).unwrap();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            DevNullCallback::<TestSvcMsgProtocol>::new_ref(),
            max_connections,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clts = ConnectionPool::new(max_connections);
        for i in 0..max_connections.get() * 2 {
            let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
                addr,
                setup::net::default_connect_timeout(),
                setup::net::default_connect_retry_after(),
                DevNullCallback::<TestCltMsgProtocol>::new_ref(),
                Some("unittest"),
            )
            .unwrap();
            info!("#{}, clt: {}", i, clt);
            if clts.has_capacity() {
                clts.add(clt).unwrap();
            } else {
                clts.add(clt).unwrap_err();
            }
            // the second half of the connections will be dropped because svc pool is at capacity
            svc.pool_accept_busywait().unwrap();
        }

        let (recv_count, send_count) = svc.len();
        info!(
            "svc: recv_count: {}, send_count: {}",
            recv_count, send_count
        );
        assert_eq!(recv_count, max_connections.get());
        assert_eq!(send_count, max_connections.get());
    }

    #[test]
    fn test_svc_clt_connected_svc_max_connections() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = NonZeroUsize::new(2).unwrap();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            DevNullCallback::<TestSvcMsgProtocol>::new_ref(),
            max_connections,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);

        let mut clts = vec![];
        for i in 0..max_connections.get() * 2 {
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

        let (recv_count, send_count) = svc.len();
        info!(
            "svc: recv_count: {}, send_count: {}",
            recv_count, send_count
        );
        assert_eq!(recv_count, max_connections.get());
        assert_eq!(send_count, max_connections.get());
    }
}
