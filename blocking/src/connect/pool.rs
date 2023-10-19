use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    num::NonZeroUsize,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    core::PoolAcceptClt,
    prelude::{AcceptClt, CallbackRecv, CallbackRecvSend, CallbackSend, Clt, CltRecver, CltSender, Messenger, RecvMsg, SendMsg, SvcAcceptor},
};
use links_core::{asserted_short_name, prelude::RoundRobinPool};

use log::{info, log_enabled, warn, Level};

/// An abstraction layer representing a pool of [Clt]'s connections
///
/// # Example
/// ```
/// use links_blocking::prelude::*;
/// use links_core::unittest::setup::{framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE}, model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug}};
/// use std::time::Duration;
///
///
/// let mut pool = CltsPool::default();
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     "127.0.0.1:8080",
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::<CltTestMessenger>::default().into(),
///     Some("unittest"),
/// );
///
/// if res.is_ok() {
///     pool.add(res.unwrap());
///
///     let mut clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
///     // Not Split for use in single thread
///     pool.send(&mut clt_msg).unwrap();
///     let svc_msg: TestSvcMsg = pool.recv().unwrap().unwrap();
///
///     // Split for use different threads
///     let ((tx_recver, tx_sender), (mut recvers, mut senders)) = pool.into_split();
///     senders.send(&mut clt_msg).unwrap();
///     let svc_msg: TestSvcMsg = recvers.recv().unwrap().unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct CltsPool<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    clts: RoundRobinPool<Clt<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> CltsPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new [CltsPool]
    /// # Arguments
    ///  * max_connections - the maximum number of connections that can be added to the pool.
    pub fn with_capacity(max_connections: NonZeroUsize) -> Self {
        Self {
            clts: RoundRobinPool::with_capacity(max_connections),
        }
    }
    /// Returns a tuple representing len of [CltRecversPool] and [CltSendersPool] respectively
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.clts.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.clts.is_empty()
    }
    #[inline(always)]
    pub fn has_capacity(&self) -> bool {
        self.clts.has_capacity()
    }
    /// Adds a [Clt] to the pool
    /// #[inline(always)]
    pub fn add(&mut self, clt: Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
        self.clts.add(clt)
    }
    #[inline(always)]
    pub fn clear(&mut self) {
        self.clts.clear();
    }
    /// Splits [CltsPool] into a a pair of channel transmitters and their respective [CltRecversPool] & [CltSendersPool] pools
    #[inline(always)]
    pub fn into_split(self) -> SplitCltsPool<M, C, MAX_MSG_SIZE> {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        let max_capacity = NonZeroUsize::new(self.clts.capacity()).unwrap();
        let mut recver_pool = CltRecversPool::new(rx_recver, max_capacity);
        let mut sender_pool = CltSendersPool::new(rx_sender, max_capacity);

        for clt in self.clts.into_iter() {
            let (clt_recver, clt_sender) = clt.into_split();
            tx_recver.send(clt_recver).expect("CltsPool::into_split - Failed to send CltRecver to CltRecversPool");
            assert!(recver_pool.service_once_rx_queue().expect("CltsPool::into_split - Failed to service CltRecversPool rx_queue"));

            tx_sender.send(clt_sender).expect("CltsPool::into_split - Failed to send CltSender to CltSendersPool");
            assert!(sender_pool.service_once_rx_queue().expect("CltsPool::into_split - Failed to service CltSendersPool rx_queue"));
        }
        ((tx_recver, tx_sender), (recver_pool, sender_pool))
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display for CltsPool<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.clts)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Default for CltsPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new [CltsPool] with a max_connections of 1
    fn default() -> Self {
        Self::with_capacity(NonZeroUsize::new(1).unwrap())
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsg<M> for CltsPool<M, C, MAX_MSG_SIZE> {
    /// Will round robin [Clt]'s in the pool to propagate the call.
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        match self.clts.round_robin() {
            Some(clt) => clt.send(msg),
            None => Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 clts available in the pool")),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsg<M> for CltsPool<M, C, MAX_MSG_SIZE> {
    /// Will round robin [Clt]'s in the pool to propagate the call.
    #[inline(always)]
    fn recv(&mut self) -> Result<Option<<M as Messenger>::RecvT>, Error> {
        match self.clts.round_robin() {
            Some(clt) => match clt.recv() {
                Ok(Some(msg)) => Ok(Some(msg)),
                Ok(None) => {
                    let clt = self.clts.remove_last_used();
                    if log_enabled!(log::Level::Info) {
                        info!("Connection reset by peer, clean. clt: {} and will be dropped, clts: {}", clt, self);
                    }
                    Ok(None)
                }
                Err(e) => {
                    let clt = self.clts.remove_last_used();
                    warn!("Connection failed, {}. clt: {} and will be dropped.  clts: {}", e, clt, self);
                    Err(e)
                }
            },
            None => Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 clts available in the pool")),
        }
    }
}
pub type SplitCltsPool<M, C, const MAX_MSG_SIZE: usize> = (
    (Sender<CltRecver<M, C, MAX_MSG_SIZE>>, Sender<CltSender<M, C, MAX_MSG_SIZE>>),
    (CltRecversPool<M, C, MAX_MSG_SIZE>, CltSendersPool<M, C, MAX_MSG_SIZE>),
);

/// A round robin pool of [CltRecver]s with respective [std::sync::mpsc::Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_blocking::prelude::*;
/// use links_core::unittest::setup::framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
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
///     DevNullCallback::<CltTestMessenger>::default().into(),
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
    recvers: RoundRobinPool<CltRecver<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecversPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new instance of [CltRecversPool]
    pub fn new(rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>, max_capacity: NonZeroUsize) -> Self {
        Self {
            rx_recver,
            recvers: RoundRobinPool::with_capacity(max_capacity),
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
            Ok(recver) => match self.recvers.add(recver) {
                Ok(_) => Ok(true),
                Err(e) => {
                    warn!("Failed to add recver to pool, {}", e);
                    Ok(false)
                }
            },
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(false),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsg<M> for CltRecversPool<M, C, MAX_MSG_SIZE> {
    /// Will round robin available recvers. If the recver connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    ///
    /// # Important
    ///
    /// This method will not check internal `rx_recver` channel for new recvers unless the pool is fully exhausted and empty.
    /// In the event there are no receivers in the channel or the pool the method will return an [Error] where `e.kind() == ErrorKind::NotConnected`
    #[inline(always)]
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error> {
        match self.recvers.round_robin() {
            Some(clt) => match clt.recv() {
                Ok(Some(msg)) => Ok(Some(msg)),
                Ok(None) => {
                    let recver = self.recvers.remove_last_used();
                    if log_enabled!(Level::Info) {
                        info!("recver: {} is dead and will be dropped, connection reset by peer. recvers: {}", recver, self);
                    }
                    Ok(None)
                }
                Err(e) => {
                    let recver = self.recvers.remove_last_used();
                    let msg = format!("recver: {} is dead and will be dropped. recvers: {} error: ({}). ", recver, self, e,);
                    Err(Error::new(e.kind(), msg))
                }
            },
            None => {
                // no receivers available try processing rx_queue
                if self.service_once_rx_queue()? {
                    self.recv()
                } else {
                    Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 recvers available in the pool"))
                }
            }
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display for CltRecversPool<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.recvers,)
    }
}

/// A round robin pool of [CltSender]s with respective [std::sync::mpsc::Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_blocking::prelude::*;
/// use links_core::unittest::setup::framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
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
///     DevNullCallback::<CltTestMessenger>::default().into(),
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
    senders: RoundRobinPool<CltSender<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSendersPool<M, C, MAX_MSG_SIZE> {
    /// Creates a new instance of [CltSendersPool]
    pub fn new(rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>, max_connections: NonZeroUsize) -> Self {
        Self {
            rx_sender,
            senders: RoundRobinPool::with_capacity(max_connections),
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
        self.senders.has_capacity()
    }
    #[inline]
    pub(crate) fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
        match self.rx_sender.try_recv() {
            Ok(sender) => match self.senders.add(sender) {
                Ok(_) => Ok(true),
                Err(e) => {
                    warn!("Failed to add sender to pool, {}", e);
                    Ok(false)
                }
            },
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(false),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsg<M> for CltSendersPool<M, C, MAX_MSG_SIZE> {
    /// Will round robin available senders. If the sender connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    ///
    /// # Important
    ///
    /// This method will not check internal `rx_sender` channel for new senders unless the pool is fully exhausted and empty.
    /// In the event there are no receivers in the channel or the pool the method will return an [Error] where `e.kind() == ErrorKind::NotConnected`
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        match self.senders.round_robin() {
            Some(clt) => match clt.send(msg) {
                Ok(s) => Ok(s),
                Err(e) => {
                    let sender = self.senders.remove_last_used();
                    let msg = format!("sender: {} is dead and will be dropped, senders: {}.  error: ({})", sender, self.senders, e);

                    Err(Error::new(e.kind(), msg))
                }
            },
            None => {
                // no senders available try processing rx_queue
                if self.service_once_rx_queue()? {
                    self.send(msg)
                } else {
                    Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 senders available in the pool"))
                }
            }
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display for CltSendersPool<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.senders)
    }
}

/// Abstraction used to accept new connections bound to the address and transmit them via a channel to the
/// respective [CltSendersPool] & [CltRecversPool].
///
/// It is designed to be used in a thread which is different from the thread that will be using the [CltSendersPool] & [CltRecversPool].
///
/// # Example
/// ```
/// use links_blocking::prelude::*;
/// use links_core::unittest::setup::framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
///
/// let addr = "127.0.0.1:8080";
/// let acceptor = SvcAcceptor::<_,_, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::<SvcTestMessenger>::default().into(),
/// );
///
/// let (tx_recver, rx_recver) = std::sync::mpsc::channel();
/// let (tx_sender, rx_sender) = std::sync::mpsc::channel();
///
/// let mut pool = PoolCltAcceptor::new(tx_recver, tx_sender, acceptor);
///
/// println!("pool: {}", pool);
///
/// pool.pool_accept().unwrap();
///
/// ```
#[derive(Debug)]
pub struct PoolCltAcceptor<M: Messenger+'static, C: CallbackRecvSend<M>+'static, const MAX_MSG_SIZE: usize> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> PoolCltAcceptor<M, C, MAX_MSG_SIZE> {
    pub fn new(tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>, tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>, acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>) -> Self {
        Self { tx_recver, tx_sender, acceptor }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> PoolAcceptClt<M, C, MAX_MSG_SIZE> for PoolCltAcceptor<M, C, MAX_MSG_SIZE> {
    /// Will interrogate the [SvcAcceptor] for new connections and if available will send them to the respective [CltRecver] & [CltSender] pools.
    fn pool_accept(&mut self) -> Result<(), Error> {
        let clt = self.acceptor.accept()?;

        let (recver, sender) = clt.into_split();
        if let Err(e) = self.tx_recver.send(recver) {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        if let Err(e) = self.tx_sender.send(sender) {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        Ok(())
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display for PoolCltAcceptor<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}>", asserted_short_name!("PoolCltAcceptor", Self), self.acceptor.con_id)
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroUsize;

    use crate::prelude::*;
    use links_core::unittest::setup::{
        self,
        framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        model::{TestCltMsg, TestCltMsgDebug},
    };

    use log::{info, LevelFilter};

    #[test]
    fn test_svcpool_cltpool_connected() {
        setup::log::configure_level(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = NonZeroUsize::new(2).unwrap();

        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, DevNullCallback::<SvcTestMessenger>::new_ref(), max_connections, Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let mut clt_pool = CltsPool::with_capacity(max_connections);
        for i in 0..max_connections.get() * 2 {
            let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
                addr,
                setup::net::default_connect_timeout(),
                setup::net::default_connect_retry_after(),
                DevNullCallback::<CltTestMessenger>::new_ref(),
                Some("unittest"),
            )
            .unwrap();
            info!("#{}, clt: {}", i, clt);
            // all connections over max_connections will be dropped
            if clt_pool.has_capacity() {
                clt_pool.add(clt).unwrap();
                svc.pool_accept().unwrap();
            } else {
                assert_eq!(clt_pool.len(), max_connections.get());
                assert_eq!(svc.pool().len(), max_connections.get());
                let clt_pool_err = clt_pool.add(clt).unwrap_err();
                info!("clt_pool_err: {:?}", clt_pool_err);
                let svc_pool_err = svc.pool_accept().unwrap_err();
                info!("svc_pool_err: {:?}", svc_pool_err);
            }
        }

        info!("clt_pool: {}", clt_pool);
        info!("svc_pool: {}", svc.pool());

        let mut clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        clt_pool.send(&mut clt_msg).unwrap();
        let svc_msg = svc.recv().unwrap().unwrap();
        info!("clt_msg: {:?}", clt_msg);
        info!("svc_msg: {:?}", svc_msg);
        assert_eq!(clt_msg, svc_msg);
    }
}
