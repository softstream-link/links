use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{core::iter::CycleRange, prelude_nonblocking::*};
use links_network_core::prelude::{CallbackRecv, CallbackRecvSend, CallbackSend, Messenger};
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
    fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
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
    /// returns true if a recver was added but only runs if there are no recivers
    #[inline]
    fn service_once_rx_queue_if_empty(&mut self) -> Result<bool, Error> {
        match self.recvers.is_empty() {
            true => self.service_once_rx_queue(),
            false => Ok(false),
        }
    }
    #[inline]
    fn next_recver(&mut self) -> Option<(usize, &mut CltRecver<M, C, MAX_MSG_SIZE>)> {
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
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Error> {
        if let Some((key, clt)) = self.next_recver() {
            match clt.recv_nonblocking() {
                Ok(ReadStatus::Completed(Some(msg))) => {
                    return Ok(ReadStatus::Completed(Some(msg)))
                }
                Ok(ReadStatus::WouldBlock) => return Ok(ReadStatus::WouldBlock),
                Ok(ReadStatus::Completed(None)) => {
                    info!(
                        "recver #{} Connection reset by peer, clean. {} and will be dropped",
                        key, self
                    );
                    self.recvers.remove(key);
                }
                Err(e) => {
                    warn!(
                        "recver #{} Failed {} and will be dropped.  error: {}",
                        key, self, e
                    );
                    self.recvers.remove(key);
                }
            };
        }

        // if we are here it means the connection was droped and we shall check the queue once more and retry
        if self.service_once_rx_queue()? {
            self.recv_nonblocking()
        } else {
            Ok(ReadStatus::WouldBlock)
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
    fn service_once_rx_queue(&mut self) -> Result<bool, Error> {
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
    fn service_once_rx_queue_if_empty(&mut self) -> Result<bool, Error> {
        match self.senders.is_empty() {
            true => self.service_once_rx_queue(),
            false => Ok(false),
        }
    }

    pub fn next_sender_mut(&mut self) -> Option<&mut CltSender<M, C, MAX_MSG_SIZE>> {
        match self.next_key_sender_mut() {
            Some((key, clt)) => Some(clt),
            None => None,
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
    fn send_nonblocking(
        &mut self,
        msg: &mut <M as Messenger>::SendT,
    ) -> Result<WriteStatus, Error> {
        if let Some((key, clt)) = self.next_key_sender_mut() {
            match clt.send_nonblocking(msg) {
                Ok(WriteStatus::Completed) => return Ok(WriteStatus::Completed),
                Ok(WriteStatus::WouldBlock) => return Ok(WriteStatus::WouldBlock),
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
        // if we are here it means the pool is empty and we shall check the queue once more and retry
        if self.service_once_rx_queue()? {
            self.send_nonblocking(msg)
        } else {
            return Ok(WriteStatus::WouldBlock);
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
