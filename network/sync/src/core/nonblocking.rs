use std::{
    error::Error,
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackSendRecvNew, MessengerNew};

use crate::connect::clt::nonblocking::Clt;

// ---- Recver ----

/// Represents the state of a non-blocking read operation
///
/// # Variants
/// * [ReadStatus::Completed(Some(T))] - indiates that read was successfull and `T` contains the value read
/// * [ReadStatus::Completed(None)] - indicates that connectioon was closed by the peer cleanly and all data was read
/// * [ReadStatus::WouldBlock] - indicates that no data was read and the caller should try again
#[derive(Debug)]
pub enum ReadStatus<T> {
    Completed(Option<T>),
    WouldBlock,
}

pub trait RecvMsgNonBlocking<M: MessengerNew> {
    /// Will attempt to read a message from the stream. Each call to this method will
    /// attemp to read data from the stream via system call and if sufficient number of bytes were read to
    /// make a single frame it will attempt to deserialize it into a message and return it
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>>;
    /// Will call [recv_nonblocking] untill it returns [ReadStatus::Completed] or
    /// will return [Err] if the call to [recv_nonblocking] returns [ReadStatus::WouldBlock] after the timeout
    fn recv_busywait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<M::RecvT>, Box<dyn Error>> {
        let start = Instant::now();
        loop {
            match self.recv_nonblocking()? {
                ReadStatus::Completed(Some(msg)) => return Ok(Some(msg)),
                ReadStatus::Completed(None) => return Ok(None),
                ReadStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Err(format!("Recv timeout: {:?}", timeout).into());
                    }
                }
            }
        }
    }
}

pub trait RecvMsgBusyWait<M: MessengerNew> {
    /// Will attempt to read a message from the stream untill there is enough bytes to make a single frame, EOF is reached or Error.
    fn recv_busywait(&mut self) -> Result<Option<M::RecvT>, Box<dyn Error>>;
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
    /// remaining bytes were written before returning [WriteStatus::Completed].
    /// [WriteStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    ///
    /// # Note
    /// Calling this method to retry might have undesired side effects and depends on the implementation.
    /// Here are a couple of examples for your condieration:
    /// * if your implementation is in fact modifying the msg then this modification will be repeated
    /// * if your implementation is logging or using a callback to propagate/collect all sent messages then it will be
    /// logged / called back twice
    fn send_nonblocking(&mut self, msg: &mut M::SendT) -> Result<WriteStatus, Box<dyn Error>>;

    /// Calls [send_nonblocking] untill it returns [WriteStatus::Completed] or
    /// will return [Err] if the call to [send_nonblocking] returns [WriteStatus::WouldBlock] after the timeout
    fn send_busywait_timeout(
        &mut self,
        msg: &mut M::SendT,
        timeout: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(()),
                WriteStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Err(format!("Send timeout: {:?}", timeout).into());
                    }
                }
            }
        }
    }
}

pub trait SendMsgNonBlocking<M: MessengerNew> {
    /// If there was a successfull attempt to write any bytes from serialized message
    /// into the stream but the write was only partial then the call shall buzy wait until all
    /// remaining bytes were written before returning [WriteStatus::Completed].
    /// [WriteStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    fn send_nonblocking(&mut self, msg: &M::SendT) -> Result<WriteStatus, Box<dyn Error>>;

    /// Calls [send_nonblocking] untill it returns [WriteStatus::Completed] or
    /// will return [Err] if the call to [send_nonblocking] returns [WriteStatus::WouldBlock] after the timeout
    fn send_nonblocking_timeout(
        &mut self,
        msg: &M::SendT,
        timeout: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let start = Instant::now();
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(()),
                WriteStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Err(format!("Send timeout: {:?}", timeout).into());
                    }
                }
            }
        }
    }
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
    /// Will attempt to accept a new connection. If there is a new connection it will return [Some(Clt)].
    /// Otherwise it will return [None] if there are no new connections to accept.
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Box<dyn Error>>;

    /// Will call [accept_nonblocking] untill it returns [Some(Clt)] or
    /// will return [Err] if the call to [accept_nonblocking] returns [None] after the timeout
    fn accept_busywait_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Box<dyn Error>> {
        let start = Instant::now();
        loop {
            match self.accept_nonblocking()? {
                Some(clt) => return Ok(Some(clt)),
                None => {
                    if start.elapsed() > timeout {
                        return Err(format!("Accept timeout: {:?}", timeout).into());
                    }
                }
            }
        }
    }
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
