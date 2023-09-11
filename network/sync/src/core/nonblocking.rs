use std::{error::Error, time::Duration};

use links_network_core::prelude::{CallbackSendRecvNew, MessengerNew};

use crate::connect::clt::nonblocking::Clt;

// ---- Recver ----

/// Represents the state of a non-blocking read operation
///
/// # Variants
///     * [ReadStatus::Completed(Some(T))] - indiates that read was successfull and `T` contains the value read
///     * [ReadStatus::Completed(None)] - indicates that connectioon was closed by the peer cleanly and all data was read
///     * [`ReadStatus::WouldBlock`] - indicates that no data was read and the caller should try again
#[derive(Debug)]
pub enum ReadStatus<T> {
    Completed(Option<T>),
    WouldBlock,
}

pub trait RecvMsgNonBlocking<M: MessengerNew> {
    /// Each call to this function
    fn recv(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>>;
}

// ---- Sender ----

/// Represents the state of the write operation
///
/// # Variants
///    * [WriteStatus::Completed] - indicates that all bytes were written to the underlying stream
///    * [WriteStatus::WouldBlock] - indicates that zero bytes were written to the underlying stream
#[derive(Debug)]
pub enum WriteStatus {
    Completed,
    WouldBlock,
}

pub trait SendMsgNonBlockingMut<M: MessengerNew> {
    /// The call will internally serialize the msg and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some bytes from serialized message
    /// into the stream but the write was only partial then the call will buzy wait until all of
    /// remaining bytes were written before returning [WriteStatus::Completed]
    /// [WriteStatus::NotReady] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    ///
    /// # Note
    /// Calling this method to retry might have undesired side effects and depends on the implementation.
    /// Here are a couple of examples for your condieration:
    /// * if your implementation is in fact modifying the msg then this modification will be repeated
    /// * if your implementation is logging or using a callback to propagate/collect all sent messages then it will be
    /// logged / called back twice
    fn send_nonblocking(&mut self, msg: &mut M::SendT) -> Result<WriteStatus, Box<dyn Error>>;
}

pub trait SendMsgNonBlocking<M: MessengerNew> {
    /// If there was a successfull attempt to write any bytes from serialized message
    /// into the stream but the write was only partial then the call shall buzy wait until all
    /// remaining bytes were written before returning [WriteStatus::Completed]
    /// [WriteStatus::NotReady] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    fn send_nonblocking(&mut self, msg: &M::SendT) -> Result<WriteStatus, Box<dyn Error>>;
}

pub trait SendMsgBusyWaitMut<M: MessengerNew> {
    /// The call will internally serialize the msg and will busy wait untill all of the bytes were written
    /// into the stream. You should never have to retry this call as it will always return [Ok(())] or non recoverable [Err]
    /// # Note
    /// * objective of this method is to not let kernel block this thread in the rate event it needs to pause writing to the stream and
    /// instead of blocking and unloading the thread from CPU it will instea busy wait untill operation succeeds
    fn send_busywait(&mut self, msg: &mut M::SendT) -> Result<(), Box<dyn Error>>;
}

// ---- Acceptor ----

pub trait AcceptCltNonBlocking<
    M: MessengerNew,
    C: CallbackSendRecvNew<M>,
    const MAX_MSG_SIZE: usize,
>
{
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Box<dyn Error>>;
}

pub trait AcceptCltBusyWait<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> {
    fn accept_busywait(&self, timeout: Duration)
        -> Result<Clt<M, C, MAX_MSG_SIZE>, Box<dyn Error>>;
}

// ---- Service Loop ----

#[derive(Debug)]
pub enum ServiceLoopStatus {
    Continue,
    Stop,
}
pub trait NonBlockingServiceLoop {
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>>;
}
