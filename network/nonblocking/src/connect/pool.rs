use std::{
    any::type_name,
    fmt::Display,
    io::{Error, ErrorKind},
    num::NonZeroUsize,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    core::iter::CycleRange,
    prelude::{
        AcceptCltNonBlocking, AcceptStatus, Acceptor, CallbackRecv, CallbackRecvSend, CallbackSend,
        Clt, CltRecver, CltSender, Messenger, NonBlockingServiceLoop, PoolAcceptCltNonBlocking,
        PoolAcceptStatus, RecvMsgNonBlocking, RecvStatus, SendMsgNonBlocking, SendStatus,
        ServiceLoopStatus,
    },
};
use log::{debug, info, log_enabled, warn};
use slab::Slab;

/// An abstraction layer representing a pool of [Clt]'s connections
///
/// # Example
/// ```
/// use links_network_nonblocking::prelude::*;
/// use links_network_core::unittest::setup::{framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE}, model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug}};
/// use std::time::Duration;
///
///
/// let mut pool = CltsPool::default();
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
#[derive(Debug)]
pub struct CltsPool<M: Messenger+'static, C: CallbackRecvSend<M>+'static, const MAX_MSG_SIZE: usize>
{
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    recver_pool: CltRecversPool<M, C, MAX_MSG_SIZE>,
    sender_pool: CltSendersPool<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> CltsPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new [CltsPool]
    /// # Arguments
    ///  * max_connections - the maximum number of connections that can be added to the pool.
    ///
    /// # Important
    ///  * The channel will continue to accept new connections even if the pool is at capacity. However, `once and only once`
    /// the [CltRecversPool] or [CltSendersPool] services its respective channel the connection will be dropped respective pool is at capacity.
    pub fn new(max_connections: NonZeroUsize) -> Self {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        Self {
            tx_recver,
            tx_sender,
            recver_pool: CltRecversPool::new(rx_recver, max_connections),
            sender_pool: CltSendersPool::new(rx_sender, max_connections),
        }
    }
    /// Returns a tuple representing len of [CltRecversPool] and [CltSendersPool] respectively
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
                format!(
                    "CltsPool recver_pool at max capacity: {}",
                    self.recver_pool.recvers.len()
                ),
            ));
        }
        if !self.sender_pool.has_capacity() {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "CltsPool sender_pool at max capacity: {}",
                    self.sender_pool.senders.len()
                ),
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
    pub fn clear(&mut self) {
        self.recver_pool.clear();
        self.sender_pool.clear();
    }
    /// Splits [CltsPool] into a a pair of channel transmitters and their respective [CltRecversPool] & [CltSendersPool] pools
    pub fn into_split(self) -> SplitCltsPool<M, C, MAX_MSG_SIZE> {
        (
            (self.tx_recver, self.tx_sender),
            (self.recver_pool, self.sender_pool),
        )
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for CltsPool<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CltsPool<{}, {}>", self.recver_pool, self.sender_pool,)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Default
    for CltsPool<M, C, MAX_MSG_SIZE>
{
    /// Creates a new [CltsPool] with a max_connections of 1
    fn default() -> Self {
        Self::new(NonZeroUsize::new(1).unwrap())
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for CltsPool<M, C, MAX_MSG_SIZE>
{
    /// Will propagate the call to [CltSendersPool::send_nonblocking] and return the result
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.sender_pool.send_nonblocking(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for CltsPool<M, C, MAX_MSG_SIZE>
{
    /// Will propagate the call to [CltRecversPool::recv_nonblocking] and return the result
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.recver_pool.recv_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for CltsPool<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.sender_pool.service_once()?;
        self.recver_pool.service_once()?;
        Ok(ServiceLoopStatus::Continue)
    }
}

pub type SplitCltsPool<M, C, const MAX_MSG_SIZE: usize> = (
    (
        Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
        Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    ),
    (
        CltRecversPool<M, C, MAX_MSG_SIZE>,
        CltSendersPool<M, C, MAX_MSG_SIZE>,
    ),
);

/// A round robin pool of [CltRecver]s with respective [std::sync::mpsc::Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_network_nonblocking::prelude::*;
/// use links_network_core::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE};
/// use std::{sync::mpsc::channel, time::Duration, num::NonZeroUsize};
///
///
/// let (tx_recver, rx_recver) = channel();
/// let mut pool = CltRecversPool::new(rx_recver, NonZeroUsize::new(2).unwrap());
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     "127.0.0.1:8080",
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::<TestCltMsgProtocol>::default().into(),
///     Some("doctest"),
/// );
///
/// if res.is_ok() {
///     let clt = res.unwrap();
///     let (recver, _sender) = clt.into_split();
///     tx_recver.send(recver);
/// }
/// ```
#[derive(Debug)]
pub struct CltRecversPool<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
    recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize>
    CltRecversPool<M, C, MAX_MSG_SIZE>
{
    /// Creates a new instance of [CltRecversPool]
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
    for CltRecversPool<M, C, MAX_MSG_SIZE>
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
    for CltRecversPool<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        let _ = self.recv_nonblocking()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for CltRecversPool<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CltRecversPool<len: {} of cap: {} [{}]>",
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

/// A round robin pool of [CltSender]s with respective [std::sync::mpsc::Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_network_nonblocking::prelude::*;
/// use links_network_core::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE};
/// use std::{sync::mpsc::channel, time::Duration, num::NonZeroUsize};
///
///
/// let (tx_recver, rx_recver) = channel();
/// let mut pool = CltSendersPool::new(rx_recver, NonZeroUsize::new(2).unwrap());
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     "127.0.0.1:8080",
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::<TestCltMsgProtocol>::default().into(),
///     Some("doctest"),
/// );
///
/// if res.is_ok() {
///     let clt = res.unwrap();
///     let (_recver, sender) = clt.into_split();
///     tx_recver.send(sender);
/// }
/// ```
#[derive(Debug)]
pub struct CltSendersPool<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
    senders: Slab<CltSender<M, C, MAX_MSG_SIZE>>,
    slab_keys: CycleRange,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize>
    CltSendersPool<M, C, MAX_MSG_SIZE>
{
    /// Creates a new instance of [CltSendersPool]
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
    for CltSendersPool<M, C, MAX_MSG_SIZE>
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
    for CltSendersPool<M, C, MAX_MSG_SIZE>
{
    #[inline]
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.service_once_rx_queue()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for CltSendersPool<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CltSendersPool<len: {} of cap: {} [{}]>",
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

/// Abstraction used to accept new connections bound to the address and transmit them via a channel to the
/// respective [CltSendersPool] & [CltRecversPool].
///
/// It is designed to be used in a thread which is different from the thread that will be using the [CltSendersPool] & [CltRecversPool].
///
/// # Example
/// ```
/// use links_network_nonblocking::prelude::*;
/// use links_network_core::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE};
///
/// let addr = "127.0.0.1:8080";
/// let acceptor = Acceptor::<_,_, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::<TestSvcMsgProtocol>::default().into(),
/// );
///
/// let (tx_recver, rx_recver) = std::sync::mpsc::channel();
/// let (tx_sender, rx_sender) = std::sync::mpsc::channel();
///
/// let mut pool = PoolCltAcceptor::new(tx_recver, tx_sender, acceptor);
///
/// println!("pool: {}", pool);
///
/// pool.pool_accept_nonblocking().unwrap();
///
/// ```
#[derive(Debug)]
pub struct PoolCltAcceptor<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    acceptor: Acceptor<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolCltAcceptor<M, C, MAX_MSG_SIZE>
{
    pub fn new(
        tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
        tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
        acceptor: Acceptor<M, C, MAX_MSG_SIZE>,
    ) -> Self {
        Self {
            tx_recver,
            tx_sender,
            acceptor,
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    PoolAcceptCltNonBlocking<M, C, MAX_MSG_SIZE> for PoolCltAcceptor<M, C, MAX_MSG_SIZE>
{
    /// Will interrogate the [Acceptor] for new connections and if available will send them to the respective [CltRecver] & [CltSender] pools.
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        match self.acceptor.accept_nonblocking()? {
            Accepted(clt) => {
                let (recver, sender) = clt.into_split();
                if let Err(e) = self.tx_recver.send(recver) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                if let Err(e) = self.tx_sender.send(sender) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                Ok(PoolAcceptStatus::Accepted)
            }
            WouldBlock => return Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for PoolCltAcceptor<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        self.pool_accept_nonblocking()?;
        Ok(ServiceLoopStatus::Continue)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for PoolCltAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = "PoolCltAcceptor";
        debug_assert_eq!(
            name,
            type_name::<Self>()
                .split("<")
                .next()
                .unwrap()
                .split("::")
                .last()
                .unwrap()
        );
        write!(f, "{} {name}", self.acceptor.con_id)
    }
}

#[cfg(test)]
mod test {
    use std::{num::NonZeroUsize, time::Duration};

    use crate::prelude::*;
    use links_network_core::{
        prelude::DevNullCallback,
        unittest::setup::{
            self,
            framer::{TestCltMsgProtocol, TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE},
            model::{TestCltMsg, TestCltMsgDebug},
        },
    };

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

        let mut clt_pool = CltsPool::new(max_connections);
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
            // all connections over max_connections will be dropped
            if clt_pool.has_capacity() {
                clt_pool.add(clt).unwrap();
                svc.pool_accept_busywait_timeout(Duration::from_millis(100))
                    .unwrap()
                    .unwrap();
            } else {
                assert_eq!(
                    clt_pool.len(),
                    (max_connections.get(), max_connections.get())
                );
                assert_eq!(
                    svc.pool().len(),
                    (max_connections.get(), max_connections.get())
                );
                let clt_pool_err = clt_pool.add(clt).unwrap_err();
                info!("clt_pool_err: {:?}", clt_pool_err);
                let svc_pool_err = svc.pool_accept_busywait().unwrap_err();
                info!("svc_pool_err: {:?}", svc_pool_err);
            }
        }

        info!("clt_pool: {}", clt_pool);
        info!("svc_pool: {}", svc.pool());

        let mut clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        clt_pool.send_busywait(&mut clt_msg).unwrap();
        let svc_msg = svc.recv_busywait().unwrap().unwrap();
        info!("clt_msg: {:?}", clt_msg);
        info!("svc_msg: {:?}", svc_msg);
        assert_eq!(clt_msg, svc_msg);
    }
}
