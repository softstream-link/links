use crate::prelude::{
    asserted_short_name, AcceptStatus, CallbackRecvSend, CltRecver, CltSender, ConnectionId, ConnectionStatus, Messenger, PollAble, PollAccept, PollRead, PoolAcceptStatus, PoolConnectionStatus, PoolSvcAcceptorOfCltNonBlocking, Protocol,
    RecvNonBlocking, RecvStatus, RoundRobinPool, SendNonBlocking, SendStatus, SvcAcceptor, SvcAcceptorOfCltNonBlocking,
};
use log::{info, log_enabled, warn, Level};
use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    marker::PhantomData,
    num::NonZeroUsize,
    sync::mpsc::{channel, Receiver, Sender},
    time::Instant,
};

use super::clt::{Clt, CltRecverRef, CltSenderRef};

pub type SplitCltsPool<M, R, S> = ((Sender<R>, Sender<S>), (CltRecversPool<M, R>, CltSendersPool<M, S>));
/// An abstraction layer representing a pool of [Clt]'s connections
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::CltTestProtocolManual};
/// use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE, model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg, SvcTestMsgDebug}};
/// use std::time::Duration;
///
///
/// let mut pool = CltsPool::default();
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     setup::net::rand_avail_addr_port(), // "127.0.0.1:9090" generates a random port
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::default().into(),
///     CltTestProtocolManual::default(),
///     Some("doctest"),
/// );
///
/// if res.is_ok() {
///     pool.add(res.unwrap());
///
///     let mut clt_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
///     // Not Split for use in single thread
///     pool.send_busywait(&mut clt_msg).unwrap();
///     let svc_msg: SvcTestMsg = pool.recv_busywait().unwrap().unwrap();
///
///     // Split for use different threads
///     let ((tx_recver, tx_sender), (mut recvers, mut senders)) = pool.into_split();
///     senders.send_busywait(&mut clt_msg).unwrap();
///     let svc_msg: SvcTestMsg = recvers.recv_busywait().unwrap().unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct CltsPool<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    clts: RoundRobinPool<Clt<P, C, MAX_MSG_SIZE>>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> CltsPool<P, C, MAX_MSG_SIZE> {
    /// Creates a new [CltsPool]
    /// # Arguments
    ///  * max_connections - the maximum number of connections that can be added to the pool.
    pub fn new(max_connections: NonZeroUsize) -> Self {
        Self { clts: RoundRobinPool::new(max_connections) }
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
    #[inline(always)]
    pub fn max_connections(&self) -> NonZeroUsize {
        self.clts.max_capacity()
    }
    /// Adds a [Clt] to the pool
    #[inline(always)]
    pub fn add(&mut self, clt: Clt<P, C, MAX_MSG_SIZE>) -> Result<(), Error> {
        self.clts.add(clt)
    }
    #[inline(always)]
    pub fn clear(&mut self) {
        self.clts.clear();
    }
    /// Splits [CltsPool] into a a pair of transmitter channels and their respective [CltRecversPool] and [CltSendersPool] pools
    /// leveraging [CltRecver] and [CltSender] respectively.
    #[inline(always)]
    pub fn into_split(self) -> SplitCltsPool<P, CltRecver<P, C, MAX_MSG_SIZE>, CltSender<P, C, MAX_MSG_SIZE>> {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        let max_capacity = self.clts.max_capacity();
        let mut recver_pool = CltRecversPool::new(rx_recver, max_capacity);
        let mut sender_pool = CltSendersPool::new(rx_sender, max_capacity);

        for clt in self.clts.into_iter() {
            let (clt_recver, clt_sender) = clt.into_split();
            tx_recver.send(clt_recver).expect("CltsPool::into_split - Failed to send CltRecver to CltRecversPool");
            assert_eq!(recver_pool.accept_into_pool().expect("CltsPool::into_split - Failed to service CltRecversPool rx_queue"), PoolAcceptStatus::Accepted);

            tx_sender.send(clt_sender).expect("CltsPool::into_split - Failed to send CltSender to CltSendersPool");
            assert_eq!(sender_pool.accept_into_pool().expect("CltsPool::into_split - Failed to service CltSendersPool rx_queue"), PoolAcceptStatus::Accepted);
        }
        ((tx_recver, tx_sender), (recver_pool, sender_pool))
    }

    #[inline(always)]
    pub fn into_split_ref(self) -> SplitCltsPool<P, CltRecverRef<P, C, MAX_MSG_SIZE>, CltSenderRef<P, C, MAX_MSG_SIZE>> {
        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();
        let max_capacity = self.clts.max_capacity();
        let mut recver_pool = CltRecversPool::new(rx_recver, max_capacity);
        let mut sender_pool = CltSendersPool::new(rx_sender, max_capacity);

        for clt in self.clts.into_iter() {
            let (clt_recver, clt_sender) = clt.into_split_ref();
            tx_recver.send(clt_recver).expect("CltsPool::into_split_ref - Failed to send CltRecver to CltRecversPool");
            assert_eq!(recver_pool.accept_into_pool().expect("CltsPool::into_split_ref - Failed to service CltRecversPool rx_queue"), PoolAcceptStatus::Accepted);

            tx_sender.send(clt_sender).expect("CltsPool::into_split_ref - Failed to send CltSender to CltSendersPool");
            assert_eq!(sender_pool.accept_into_pool().expect("CltsPool::into_split_ref - Failed to service CltSendersPool rx_queue"), PoolAcceptStatus::Accepted);
        }
        ((tx_recver, tx_sender), (recver_pool, sender_pool))
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for CltsPool<P, C, MAX_MSG_SIZE> {
    /// Will round robin [Clt]'s in the pool to propagate the call.
    ///
    /// # Important
    ///
    /// Will return [Err(ErrorKind::NotConnected)] if the pool is empty, so that the [Self::send_busywait] does not block indefinitely.
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        match self.clts.round_robin() {
            Some(clt) => clt.send(msg),
            None => Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 clts available in the pool")),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for CltsPool<P, C, MAX_MSG_SIZE> {
    /// Will round robin [Clt]'s in the pool to propagate the call.
    /// Will return [Err(ErrorKind::NotConnected)] if the pool is empty, so that the [Self::recv_busywait] does not block indefinitely.
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        match self.clts.round_robin() {
            Some(clt) => match clt.recv() {
                Ok(Completed(Some(msg))) => Ok(Completed(Some(msg))),
                Ok(WouldBlock) => Ok(WouldBlock),
                Ok(Completed(None)) => {
                    let clt = self.clts.remove_last_used();
                    if log_enabled!(log::Level::Info) {
                        info!("Connection reset by peer, clean. clt: {} and will be dropped, clts: {}", clt, self);
                    }
                    Ok(RecvStatus::Completed(None))
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
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolConnectionStatus for CltsPool<P, C, MAX_MSG_SIZE> {
    /// Will only test connection status of the next [Clt] in the pool that will be used to service [SendNonBlocking::send] or [RecvNonBlocking::recv]
    #[inline(always)]
    fn is_next_connected(&mut self) -> bool {
        match self.clts.current() {
            Some(clt) => clt.is_connected(),
            None => false,
        }
    }
    #[inline(always)]
    fn all_connected(&mut self) -> bool {
        if self.clts.is_empty() {
            return false;
        }
        for (_k, clt) in self.clts.iter() {
            if !clt.is_connected() {
                return false;
            }
        }
        true
    }
}

impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for CltsPool<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<<P as Messenger>::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<<P as Messenger>::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<RecvP:{}, SendP:{}, {}>", asserted_short_name!("CltsPool", Self), recv_t, send_t, self.clts)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Default for CltsPool<P, C, MAX_MSG_SIZE> {
    /// Creates a new [CltsPool] with a max_connections of 1
    fn default() -> Self {
        Self::new(NonZeroUsize::new(1).unwrap())
    }
}

/// A round robin pool of [CltRecver]s with respective [Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::CltTestProtocolManual};
/// use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE};
/// use std::{sync::mpsc::channel, time::Duration, num::NonZeroUsize};
///
///
/// let (tx_recver, rx_recver) = channel();
/// let mut pool = CltRecversPool::new(rx_recver, NonZeroUsize::new(2).unwrap());
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     setup::net::rand_avail_addr_port(), // "127.0.0.1:8080" generates a random port
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::default().into(),
///     CltTestProtocolManual::default(),
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
pub struct CltRecversPool<M: Messenger + 'static, R: RecvNonBlocking<M> + ConnectionStatus> {
    rx_recver: Receiver<R>,
    recvers: RoundRobinPool<R>,
    phantom: PhantomData<M>,
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> CltRecversPool<M, R> {
    /// Creates a new instance of [CltRecversPool]
    pub fn new(rx_recver: Receiver<R>, max_connections: NonZeroUsize) -> Self {
        Self {
            rx_recver,
            recvers: RoundRobinPool::new(max_connections),
            phantom: PhantomData,
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
        self.recvers.has_capacity()
    }
    pub fn max_connection(&self) -> NonZeroUsize {
        self.recvers.max_capacity()
    }
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> SvcAcceptorOfCltNonBlocking<R> for CltRecversPool<M, R> {
    /// Will interrogate internal [channel] for new [CltRecver]s.
    /// # Returns
    /// * [Ok(AcceptStatus::Accepted(Some))] - if a new [CltRecver] is available
    /// * [Ok(AcceptStatus::WouldBlock)] - if no new [CltRecver] is available
    /// * [Err(Error::Other)] - if the [Sender] part of [channel] has been dropped
    #[inline(always)]
    fn accept(&self) -> Result<AcceptStatus<R>, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        match self.rx_recver.try_recv() {
            Ok(recver) => Ok(Accepted(recver)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(WouldBlock),
            Err(e) => Err(Error::new(ErrorKind::Other, format!("Channel can no longer accept recvers, {}", e))),
        }
    }
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> PoolSvcAcceptorOfCltNonBlocking for CltRecversPool<M, R> {
    /// Will `once ` interrogate internal [channel] for a new [CltRecver] and add it to the connection pool if there is capacity.
    /// Otherwise the [CltRecver] will be dropped and [Ok(PoolAcceptStatus::WouldBlock)] returned
    fn accept_into_pool(&mut self) -> Result<PoolAcceptStatus, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept()? {
            Accepted(recver) => match self.recvers.add(recver) {
                Ok(_) => Ok(PoolAcceptStatus::Accepted),
                Err(e) => {
                    warn!("Failed to add recver to pool, {}", e);
                    Ok(PoolAcceptStatus::WouldBlock)
                }
            },
            Rejected => Ok(PoolAcceptStatus::Rejected),
            WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> RecvNonBlocking<M> for CltRecversPool<M, R> {
    /// Will round robin [CltRecver]'s in the pool to propagate the call.
    /// If the recver connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    /// Each call to this method will result in a call to [PoolSvcAcceptorOfCltNonBlocking::accept_into_pool].
    ///
    /// # Important
    ///
    /// In the event there are no [CltRecver] in the channel or the pool the method will return an [Err(ErrorKind::NotConnected)]
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        match self.recvers.round_robin() {
            Some(clt) => match clt.recv() {
                Ok(Completed(Some(msg))) => {
                    self.accept_into_pool()?;
                    Ok(Completed(Some(msg)))
                }
                Ok(WouldBlock) => {
                    self.accept_into_pool()?;
                    Ok(WouldBlock)
                }
                Ok(Completed(None)) => {
                    let recver = self.recvers.remove_last_used();
                    if log_enabled!(Level::Info) {
                        info!("recver: {} is dead and will be dropped, connection reset by peer. recvers: {}", recver, self);
                    }
                    self.accept_into_pool()?;
                    Ok(Completed(None))
                }
                Err(e) => {
                    let recver = self.recvers.remove_last_used();
                    self.accept_into_pool()?;
                    Err(Error::new(e.kind(), format!("recver: {} is dead and will be dropped. recvers: {} error: ({}). ", recver, self, e,)))
                }
            },
            None => {
                // no receivers available try processing rx_queue
                if let PoolAcceptStatus::Accepted = self.accept_into_pool()? {
                    self.recv()
                } else {
                    Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 recvers available in the pool"))
                }
            }
        }
    }
    /// Will call [Self::recv] in a loop until the message is received or an error is returned.
    ///
    /// # Important
    ///
    /// * In the event there are no [CltRecver] in the channel and the pool is empty the method will continue to call [Self::recv] until timeout,
    /// hoping that a new [CltRecver] will be added to the pool.
    #[inline(always)]
    fn recv_busywait_timeout(&mut self, timeout: std::time::Duration) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            match self.recv() {
                Ok(Completed(opt)) => return Ok(Completed(opt)),
                Ok(WouldBlock) => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                    continue;
                }
                // only raised when pool is empty
                Err(e) if e.kind() == ErrorKind::NotConnected => {
                    if start.elapsed() > timeout {
                        return Err(e);
                    }
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
    /// Will call [Self::recv] in a loop until the message is received or an error is returned.
    ///
    /// # Important
    ///
    /// * The call will block indefinitely if the pool is empty.
    #[inline(always)]
    fn recv_busywait(&mut self) -> Result<Option<<M as Messenger>::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        loop {
            match self.recv() {
                Ok(Completed(opt)) => return Ok(opt),
                Ok(WouldBlock) => continue,
                // only raised when pool is empty
                Err(e) if e.kind() == ErrorKind::NotConnected => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> PoolConnectionStatus for CltRecversPool<M, R> {
    /// Will test connection status of the next [CltRecver] in the pool that will be used to service [RecvNonBlocking::recv]
    ///
    /// # Important
    /// This method will integrate and test first [CltRecver] in the `rx_recver` channel if the pool is empty
    #[inline(always)]
    fn is_next_connected(&mut self) -> bool {
        match self.recvers.current() {
            Some(clt) => clt.is_connected(),
            None => {
                if let PoolAcceptStatus::Accepted = self.accept_into_pool().expect("CltRecversPool::accept_into_pool - Failed to service rx_recver, was the tx_recver dropped?") {
                    self.is_next_connected()
                } else {
                    false
                }
            }
        }
    }
    #[inline(always)]
    /// Will test connection status of all [CltRecver]s in the pool including the first [CltRecver] in the `rx_recver` channel if the pool is empty
    fn all_connected(&mut self) -> bool {
        if self.recvers.is_empty() {
            if let PoolAcceptStatus::Accepted = self.accept_into_pool().expect("CltRecversPool::accept_into_pool - Failed to service rx_recver, was the tx_recver dropped?") {
                return self.all_connected();
            } else {
                return false;
            }
        }
        for (_k, recver) in self.recvers.iter() {
            if !recver.is_connected() {
                return false;
            }
        }
        true
    }
}
impl<M: Messenger, R: RecvNonBlocking<M> + ConnectionStatus> Display for CltRecversPool<M, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<RecvP:{}, SendP:{}, {}>", asserted_short_name!("CltRecversPool", Self), recv_t, send_t, self.recvers)
    }
}

/// A round robin pool of [CltSender]s with respective [std::sync::mpsc::Receiver] channel
/// though which the pool can be populated.
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::CltTestProtocolManual};
/// use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE};
/// use std::{sync::mpsc::channel, time::Duration, num::NonZeroUsize};
///
///
/// let (tx_recver, rx_recver) = channel();
/// let mut pool = CltSendersPool::new(rx_recver, NonZeroUsize::new(2).unwrap());
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     setup::net::rand_avail_addr_port(), // "127.0.0.1:8080" generates a random port
///     Duration::from_millis(100),
///     Duration::from_millis(10),
///     DevNullCallback::default().into(),
///     CltTestProtocolManual::default(),
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
pub struct CltSendersPool<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> {
    rx_sender: Receiver<S>,
    senders: RoundRobinPool<S>,
    phantom: PhantomData<M>,
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> CltSendersPool<M, S> {
    /// Creates a new instance of [CltSendersPool]
    pub fn new(rx_sender: Receiver<S>, max_connections: NonZeroUsize) -> Self {
        Self {
            rx_sender,
            senders: RoundRobinPool::new(max_connections),
            phantom: PhantomData,
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
    pub fn max_connection(&self) -> NonZeroUsize {
        self.senders.max_capacity()
    }
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> SvcAcceptorOfCltNonBlocking<S> for CltSendersPool<M, S> {
    /// Will interrogate internal [channel] for new [CltSender]s.
    /// # Returns
    /// * [Ok(AcceptStatus::Accepted(Some))] - if a new [CltSender] is available
    /// * [Ok(AcceptStatus::WouldBlock)] - if no new [CltSender] is available
    /// * [Err(Error::Other)] - if the [Sender] part of [channel] has been dropped
    #[inline(always)]
    fn accept(&self) -> Result<AcceptStatus<S>, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        match self.rx_sender.try_recv() {
            Ok(sender) => Ok(Accepted(sender)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(WouldBlock),
            Err(e) => Err(Error::new(ErrorKind::Other, format!("Channel can no longer accept senders, {}", e))),
        }
    }
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> PoolSvcAcceptorOfCltNonBlocking for CltSendersPool<M, S> {
    /// Will `once ` interrogate internal [channel] for a new [CltSender] and add it to the connection pool if there is capacity.
    /// Otherwise the [CltSender] will be dropped and [Ok(PoolAcceptStatus::WouldBlock)] returned
    #[inline(always)]
    fn accept_into_pool(&mut self) -> Result<PoolAcceptStatus, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept()? {
            Accepted(sender) => match self.senders.add(sender) {
                Ok(_) => Ok(PoolAcceptStatus::Accepted),
                Err(e) => {
                    warn!("Failed to add sender to pool, {}", e);
                    Ok(PoolAcceptStatus::WouldBlock)
                }
            },
            Rejected => Ok(PoolAcceptStatus::Rejected),
            WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> SendNonBlocking<M> for CltSendersPool<M, S> {
    /// Will round robin [CltSender]'s in the pool to propagate the call.
    /// If the sender connection is dead it will be removed and relevant error propagated.
    /// In order to try next recver the caller must call this method again.
    /// Each call to this method will result in a call to [PoolSvcAcceptorOfCltNonBlocking::accept_into_pool].
    ///
    /// # Important
    ///
    /// * In the event there are no [CltSender] in the channel and the pool is empty the method will return an [Err(ErrorKind::NotConnected)]
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        // 1. get next
        // 1.a. if Some()
        //  2.a. if Ok run pool_accept once and return Ok
        //  2.b. if Err remove it, run pool_accept once, and return Err
        // 1.b. if None
        //  2.a. run pool_accept once and try send again
        match self.senders.round_robin() {
            Some(s) => match s.send(msg) {
                Ok(s) => {
                    self.accept_into_pool()?;
                    Ok(s)
                }
                Err(e) => {
                    let sender = self.senders.remove_last_used();
                    self.accept_into_pool()?;
                    Err(Error::new(e.kind(), format!("sender: {} is dead and will be dropped, senders: {}.  error: ({})", sender, self.senders, e)))
                }
            },
            None => {
                // no senders available try processing rx_queue
                if let PoolAcceptStatus::Accepted = self.accept_into_pool()? {
                    self.send(msg)
                } else {
                    Err(Error::new(ErrorKind::NotConnected, "Not Connected, 0 senders available in the pool"))
                }
            }
        }
    }
    /// Will call [Self::send] in a loop until the message is sent or an error is returned.
    ///
    /// # Important
    ///
    /// * In the event there are no [CltSender] in the channel and the pool is empty the method will continue to call [Self::send] until timeout,
    /// hoping that a new [CltSender] will be added to the pool.
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <M as Messenger>::SendT, timeout: std::time::Duration) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            match self.send(msg) {
                Ok(Completed) => return Ok(Completed),
                Ok(WouldBlock) => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                    continue;
                }
                // only raised when pool is empty
                Err(e) if e.kind() == ErrorKind::NotConnected => {
                    if start.elapsed() > timeout {
                        return Err(e);
                    }

                    // info!("sending busy_wait_timeout not timeout yet");
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
    /// Will call [Self::send] in a loop until the message is sent or an error is returned.
    ///
    /// # Important
    ///
    /// * The call will block indefinitely if the pool is empty.
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        loop {
            match self.send(msg) {
                Ok(Completed) => return Ok(()),
                Ok(WouldBlock) => continue,
                Err(e) if e.kind() == ErrorKind::NotConnected => continue, // only raised when pool is empty
                Err(e) => return Err(e),
            }
        }
    }
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> PoolConnectionStatus for CltSendersPool<M, S> {
    /// Will test connection status of the next [CltSender] in the pool that will be used to service [SendNonBlocking::send]
    ///
    /// # Important
    /// This method will integrate and test first [CltSender] in the `rx_sender` channel if the pool is empty
    #[inline(always)]
    fn is_next_connected(&mut self) -> bool {
        match self.senders.current() {
            Some(s) => s.is_connected(),
            None => {
                if let PoolAcceptStatus::Accepted = self.accept_into_pool().expect("CltSendersPool::accept_into_pool - Failed to service rx_sender, was the tx_sender dropped?") {
                    self.is_next_connected()
                } else {
                    false
                }
            }
        }
    }
    #[inline(always)]
    /// Will test connection status of all [CltSender]s in the pool including the first [CltSender] in the `rx_sender` channel if the pool is empty
    fn all_connected(&mut self) -> bool {
        if self.senders.is_empty() {
            if let PoolAcceptStatus::Accepted = self.accept_into_pool().expect("CltSendersPool::accept_into_pool - Failed to service rx_sender, was the tx_sender dropped?") {
                return self.all_connected();
            } else {
                return false;
            }
        }
        for (_k, sender) in self.senders.iter() {
            if !sender.is_connected() {
                return false;
            }
        }
        true
    }
}
impl<M: Messenger, S: SendNonBlocking<M> + ConnectionStatus> Display for CltSendersPool<M, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<RecvP:{}, SendP:{}, {}>", asserted_short_name!("CltSendersPool", Self), recv_t, send_t, self.senders)
    }
}

/// Abstraction uses underlying [SvcAcceptor] to accept new [Clt] which are then split into [CltRecver] and [CltSender]
/// and transmitted via respective channel
///
/// It is designed to be used in a [crate::prelude::PollHandler] thread which can be different from the thread that will be using the [CltSendersPool] & [CltRecversPool].
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::{CltTestProtocolManual, SvcTestProtocolManual}};
/// use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
///
/// let addr = setup::net::rand_avail_addr_port(); // will return random port "127.0.0.1:8080"
/// let acceptor = SvcAcceptor::<_,_, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::default().into(),
///     SvcTestProtocolManual::default(),
///     NonZeroUsize::new(1).unwrap(),
/// );
///
/// let (tx_recver, rx_recver) = std::sync::mpsc::channel();
/// let (tx_sender, rx_sender) = std::sync::mpsc::channel();
///
/// let mut acceptor = TransmittingSvcAcceptor::new(tx_recver, tx_sender, acceptor);
///
/// println!("acceptor: {}", acceptor);
///
/// assert_eq!(acceptor.accept_into_pool().unwrap(),  PoolAcceptStatus::WouldBlock);
///
/// // Create a new
/// let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     addr,
///     setup::net::default_connect_timeout(),
///     setup::net::default_connect_retry_after(),
///     DevNullCallback::default().into(),
///     CltTestProtocolManual::default(),
///     Some("unittest")).unwrap();
///
/// let res = acceptor.accept_into_pool();
/// println!("res: {:?}", res);
/// assert_eq!(res.unwrap(),  PoolAcceptStatus::Accepted);
/// //assert!(false); // uncomment to see output
/// ```
#[derive(Debug)]
pub struct TransmittingSvcAcceptor<P: Protocol + 'static, C: CallbackRecvSend<P> + 'static, const MAX_MSG_SIZE: usize> {
    tx_recver: Sender<CltRecver<P, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<P, C, MAX_MSG_SIZE>>,
    acceptor: SvcAcceptor<P, C, MAX_MSG_SIZE>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    pub fn new(tx_recver: Sender<CltRecver<P, C, MAX_MSG_SIZE>>, tx_sender: Sender<CltSender<P, C, MAX_MSG_SIZE>>, acceptor: SvcAcceptor<P, C, MAX_MSG_SIZE>) -> Self {
        Self { tx_recver, tx_sender, acceptor }
    }
    /// Will interrogate the [SvcAcceptor] for new connections and if available will return [CltRecver] and send [CltSender] to the respective [CltSender] pools.
    pub fn accept_recver(&mut self) -> Result<AcceptStatus<CltRecver<P, C, MAX_MSG_SIZE>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.acceptor.accept()? {
            Accepted(clt) => {
                let (recver, sender) = clt.into_split();
                if let Err(e) = self.tx_sender.send(sender) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                Ok(Accepted(recver))
            }
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolSvcAcceptorOfCltNonBlocking for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    /// Will interrogate the [SvcAcceptor] for new connections and if available will send them to the respective [CltRecver] & [CltSender] pools.
    fn accept_into_pool(&mut self) -> Result<PoolAcceptStatus, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.acceptor.accept()? {
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
            Rejected => Ok(PoolAcceptStatus::Rejected),
            WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAble for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        Box::new(&mut self.acceptor.listener)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn con_id(&self) -> &links_core::prelude::ConId {
        &self.acceptor.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAccept<CltRecver<P, C, MAX_MSG_SIZE>> for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<CltRecver<P, C, MAX_MSG_SIZE>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept_recver()? {
            Accepted(recver) => Ok(Accepted(recver)),
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAccept<Box<dyn PollRead>> for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<Box<dyn PollRead>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept_recver()? {
            Accepted(recver) => Ok(Accepted(Box::new(recver))),
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}>", asserted_short_name!("TransmittingSvcAcceptor", Self), self.acceptor.con_id)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> From<TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE>> for Box<dyn PollAccept<Box<dyn PollRead>>> {
    fn from(value: TransmittingSvcAcceptor<P, C, MAX_MSG_SIZE>) -> Self {
        Box::new(value)
    }
}

/// Abstraction uses underlying [SvcAcceptor] to accept new [Clt] which are then split into [CltRecverRef] and [CltSenderRef]
/// and transmitted via respective channel
///
/// It is designed to be used in a [crate::prelude::PollHandler] thread which can be different from the thread that will be using the [CltSendersPool] & [CltRecversPool].
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::{CltTestProtocolManual, SvcTestProtocolManual}};
/// use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE};
/// use std::num::NonZeroUsize;
///
/// let addr = setup::net::rand_avail_addr_port(); // will return random port "127.0.0.1:8080"
/// let acceptor = SvcAcceptor::<_,_, TEST_MSG_FRAME_SIZE>::new(
///     ConId::svc(Some("doctest"), addr, None),
///     std::net::TcpListener::bind(addr).unwrap(),
///     DevNullCallback::default().into(),
///     SvcTestProtocolManual::default(),
///     NonZeroUsize::new(1).unwrap(),
/// );
///
/// let (tx_recver, rx_recver) = std::sync::mpsc::channel();
/// let (tx_sender, rx_sender) = std::sync::mpsc::channel();
///
/// let mut acceptor = TransmittingSvcAcceptorRef::new(tx_recver, tx_sender, acceptor);
///
/// println!("acceptor: {}", acceptor);
///
/// assert_eq!(acceptor.accept_into_pool().unwrap(),  PoolAcceptStatus::WouldBlock);
///
/// // Create a new
/// let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///     addr,
///     setup::net::default_connect_timeout(),
///     setup::net::default_connect_retry_after(),
///     DevNullCallback::default().into(),
///     CltTestProtocolManual::default(),
///     Some("unittest")).unwrap();
///
/// let res = acceptor.accept_into_pool();
/// println!("res: {:?}", res);
/// assert_eq!(res.unwrap(),  PoolAcceptStatus::Accepted);
/// //assert!(false); // uncomment to see output
/// ```
#[derive(Debug)]
pub struct TransmittingSvcAcceptorRef<P: Protocol + 'static, C: CallbackRecvSend<P> + 'static, const MAX_MSG_SIZE: usize> {
    tx_recver: Sender<CltRecverRef<P, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSenderRef<P, C, MAX_MSG_SIZE>>,
    acceptor: SvcAcceptor<P, C, MAX_MSG_SIZE>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    pub fn new(tx_recver: Sender<CltRecverRef<P, C, MAX_MSG_SIZE>>, tx_sender: Sender<CltSenderRef<P, C, MAX_MSG_SIZE>>, acceptor: SvcAcceptor<P, C, MAX_MSG_SIZE>) -> Self {
        Self { tx_recver, tx_sender, acceptor }
    }
    /// Will interrogate the [SvcAcceptor] for new connections and if available will return [CltRecver] and send [CltSender] to the respective [CltSender] pools.
    pub fn accept_recver(&mut self) -> Result<AcceptStatus<CltRecverRef<P, C, MAX_MSG_SIZE>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.acceptor.accept()? {
            Accepted(clt) => {
                let (recver, sender) = clt.into_split_ref();

                if let Err(e) = self.tx_sender.send(sender) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                Ok(Accepted(recver))
            }
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PoolSvcAcceptorOfCltNonBlocking for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    /// Will interrogate the [SvcAcceptor] for new connections and if available will send them to the respective [CltRecver] & [CltSender] pools.
    fn accept_into_pool(&mut self) -> Result<PoolAcceptStatus, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.acceptor.accept()? {
            Accepted(clt) => {
                let (recver, sender) = clt.into_split_ref();
                if let Err(e) = self.tx_recver.send(recver) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }

                if let Err(e) = self.tx_sender.send(sender) {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                Ok(PoolAcceptStatus::Accepted)
            }
            Rejected => Ok(PoolAcceptStatus::Rejected),
            WouldBlock => Ok(PoolAcceptStatus::WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAble for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        Box::new(&mut self.acceptor.listener)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    fn con_id(&self) -> &links_core::prelude::ConId {
        &self.acceptor.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAccept<CltRecverRef<P, C, MAX_MSG_SIZE>> for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<CltRecverRef<P, C, MAX_MSG_SIZE>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept_recver()? {
            Accepted(recver) => Ok(Accepted(recver)),
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollAccept<Box<dyn PollRead>> for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    fn poll_accept(&mut self) -> Result<AcceptStatus<Box<dyn PollRead>>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        match self.accept_recver()? {
            Accepted(recver) => Ok(Accepted(Box::new(recver))),
            Rejected => Ok(Rejected),
            WouldBlock => Ok(WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}>", asserted_short_name!("TransmittingSvcAcceptorRef", Self), self.acceptor.con_id)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> From<TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE>> for Box<dyn PollAccept<Box<dyn PollRead>>> {
    fn from(value: TransmittingSvcAcceptorRef<P, C, MAX_MSG_SIZE>) -> Self {
        Box::new(value)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::prelude::*;
    use crate::unittest::setup::protocol::{CltTestProtocolManual, SvcTestProtocolManual};
    use links_core::unittest::setup::{
        self,
        framer::TEST_MSG_FRAME_SIZE,
        model::{CltTestMsg, CltTestMsgDebug},
    };
    use std::{io::ErrorKind, num::NonZeroUsize, time::Duration};

    use log::{info, LevelFilter};

    #[test]
    fn test_svcpool_cltpool_connected() {
        setup::log::configure_compact(LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();
        let max_connections = NonZeroUsize::new(2).unwrap();
        let mut svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, DevNullCallback::new_ref(), max_connections, SvcTestProtocolManual::default(), Some("unittest")).unwrap();
        info!("svc: {}", svc);

        let mut clt_pool = CltsPool::new(max_connections);
        for i in 0..max_connections.get() * 2 {
            let clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
                addr,
                setup::net::default_connect_timeout(),
                setup::net::default_connect_retry_after(),
                DevNullCallback::new_ref(),
                CltTestProtocolManual::default(),
                Some("unittest"),
            )
            .unwrap();
            info!("#{}, clt: {}", i, clt);
            // all connections over max_connections will be dropped
            if clt_pool.has_capacity() {
                clt_pool.add(clt).unwrap();
                svc.accept_into_pool_busywait_timeout(Duration::from_millis(100)).unwrap().unwrap_accepted();
            } else {
                assert_eq!(clt_pool.len(), max_connections.get());
                assert_eq!(svc.pool().len(), max_connections.get());
                let clt_pool_err = clt_pool.add(clt).unwrap_err();
                info!("clt_pool_err: {:?}", clt_pool_err);
                let svc_pool_err = svc.accept_into_pool_busywait().unwrap().unwrap_rejected();
                info!("svc_pool_err: {:?}", svc_pool_err);
            }
        }

        info!("clt_pool: {}", clt_pool);
        info!("svc_pool: {}", svc.pool());

        let mut clt_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
        clt_pool.send_busywait(&mut clt_msg).unwrap();
        let svc_msg = svc.recv_busywait().unwrap().unwrap();
        info!("clt_msg: {:?}", clt_msg);
        info!("svc_msg: {:?}", svc_msg);
        assert_eq!(clt_msg, svc_msg);

        // cover error cases when pool is empty
        clt_pool.clear();
        info!("clt_pool: {}", clt_pool);
        assert_eq!(clt_pool.len(), 0);

        // clt_pool generates ErrorKind::NotConnected when empty
        let res = clt_pool.send_busywait(&mut clt_msg);
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::NotConnected);
        let res = clt_pool.recv_busywait();
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::NotConnected);

        let ((_tx_recver, _tx_sender), (mut clt_recv, mut clt_send)) = clt_pool.into_split();
        info!("clt_recv: {}", clt_recv);
        info!("clt_send: {}", clt_send);

        // IMPORTANT unlike clt_pool clt_recv and clt_send will block on busy_wait calls since it is possible to accept a new connection while waiting
        let res = clt_recv.recv_busywait_timeout(Duration::from_millis(100));
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::NotConnected);

        // IMPORTANT unlike clt_pool clt_recv and clt_send will block on busy_wait calls since it is possible to accept a new connection while waiting
        let res = clt_send.send_busywait_timeout(&mut clt_msg, Duration::from_millis(100));
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::NotConnected);

        // test that pool_accept error is propagated
        drop(_tx_recver);
        let res = clt_recv.recv_busywait();
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::Other);

        // test that pool_accept error is propagated
        drop(_tx_sender);
        let res = clt_send.send_busywait(&mut clt_msg);
        info!("res: {:?}", res);
        assert_eq!(res.unwrap_err().kind(), ErrorKind::Other);
    }
}
