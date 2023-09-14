use std::{
    io::{Error, ErrorKind},
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackSendRecv, Messenger};

use crate::prelude_nonblocking::Clt;

// ---- Acceptor ----

pub enum AcceptStatus<T> {
    Accepted(Option<T>),
    WouldBlock,
}

pub trait AcceptCltNonBlocking<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> {
    // fn accept_nonblockings(&self) -> Result<(), Error>;

    /// Will attempt to accept a new connection. If there is a new connection it will return [Some(Clt)].
    /// Otherwise it will return [None] if there are no new connections to accept.
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error>;

    /// Will call [accept_nonblocking] untill it returns [Some(Clt)] or
    /// will return [Err(e)] where [e.kind() == ErrorKind::TimeOut] if the call to [accept_nonblocking] returns [None] after the timeout
    fn accept_busywait_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        let start = Instant::now();
        loop {
            match self.accept_nonblocking()? {
                Some(clt) => return Ok(Some(clt)),
                None => {
                    if start.elapsed() > timeout {
                        return Err(Error::new(
                            ErrorKind::TimedOut,
                            format!("Accept timeout: {:?}", timeout),
                        ));
                    }
                }
            }
        }
    }
}

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
impl<T> ReadStatus<T> {
    /// Will panic if the variant is [ReadStatus::WouldBlock], otherwise unwraps into [Option<T>] from [ReadStatus::Completed(Option<T>)]
    pub fn unwrap(self) -> Option<T> {
        match self {
            ReadStatus::Completed(o) => o,
            ReadStatus::WouldBlock => panic!("ReadStatus::WouldBlock"),
        }
    }
    pub fn is_completed(&self) -> bool {
        match self {
            ReadStatus::Completed(_) => true,
            ReadStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        !self.is_completed()
    }
}

pub trait RecvMsgNonBlocking<M: Messenger> {
    /// Will attempt to read a message from the stream. Each call to this method will
    /// attemp to read data from the stream via system call and if sufficient number of bytes were read to
    /// make a single frame it will attempt to deserialize it into a message and return it
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Error>;

    /// Will call [recv_nonblocking] untill it returns [ReadStatus::Completed] or [ReadStatus::WouldBlock] after the timeoutok,
    fn recv_busywait_timeout(&mut self, timeout: Duration) -> Result<ReadStatus<M::RecvT>, Error> {
        let start = Instant::now();
        loop {
            match self.recv_nonblocking()? {
                ReadStatus::Completed(o) => return Ok(ReadStatus::Completed(o)),
                ReadStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(ReadStatus::WouldBlock);
                    }
                }
            }
        }
    }
    /// Will busywait block on [recv_nonblocking] untill it returns [ReadStatus::Completed]
    fn recv_busywait(&mut self) -> Result<Option<M::RecvT>, Error> {
        loop {
            match self.recv_nonblocking()? {
                ReadStatus::Completed(o) => return Ok(o),
                ReadStatus::WouldBlock => continue,
            }
        }
    }
}

// pub trait RecvMsgBusyWait<M: Messenger> {
//     /// Will attempt to read a message from the stream untill there is enough bytes to make a single frame, EOF is reached or Error.
//     fn recv_busywait(&mut self) -> Result<Option<M::RecvT>, Error>;
// }

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

pub trait SendMsgNonBlockingMut<M: Messenger> {
    /// The call will internally serialize the msg and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some bytes from serialized message
    /// into the stream but the write was only partial then the call will buzy wait until all of
    /// remaining bytes were written before returning [WriteStatus::Completed].
    /// [WriteStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    ///
    /// # Warning!
    /// Calling this method second time after [WriteStatus::WouldBlock] `might have undesired side effects`
    ///  and depends on the implementation.
    /// Here are a couple of examples for your condieration:
    /// * if your implementation is in modifying the msg then this modification will be repeated
    /// * if your implementation is logging or using a callback to propagate/collect all sent messages then it will be
    /// logged / called back twice
    ///
    /// To Avoid these effects implement and use [send_busywait] instead which allows you to perform all modification and callbacks
    /// a single time before commiting to a busy wait loop while sending the message
    fn send_nonblocking(&mut self, msg: &mut M::SendT) -> Result<WriteStatus, Error>;

    /// Implement and use this metthod Use this impWill busywait block on [send_nonblocking] untill it returns [WriteStatus::Completed]
    fn send_busywait(&mut self, msg: &mut M::SendT) -> Result<(), Error>;

    /// Will call [send_nonblocking] untill it returns [WriteStatus::Completed] or [WriteStatus::WouldBlock] after the timeoutok,
    #[inline(always)]
    fn send_busywait_timeout(
        &mut self,
        msg: &mut M::SendT,
        timeout: Duration,
    ) -> Result<WriteStatus, Error> {
        let start = Instant::now();
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(WriteStatus::Completed),
                WriteStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WriteStatus::WouldBlock);
                    }
                }
            }
        }
    }
}

/// # Warning!!!
/// This is not a public trait as it [send_nonblocking] implementaiton requires special consideration if it involves modification of state as it is expected
/// to be called repetedly to recover from [WriteStatus::WouldBlock]
pub(crate) trait SendMsgNonBlocking<M: Messenger> {
    /// If there was a successfull attempt to write any bytes from serialized message
    /// into the stream but the write was only partial then the call shall buzy wait until all
    /// remaining bytes were written before returning [WriteStatus::Completed].
    /// [WriteStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    fn send_nonblocking(&mut self, msg: &M::SendT) -> Result<WriteStatus, Error>;

    /// Will call [send_nonblocking] untill it returns [WriteStatus::Completed] or [WriteStatus::WouldBlock] after the timeoutok,
    #[inline(always)]
    fn send_nonblocking_timeout(
        &mut self,
        msg: &M::SendT,
        timeout: Duration,
    ) -> Result<WriteStatus, Error> {
        let start = Instant::now();
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(WriteStatus::Completed),
                WriteStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WriteStatus::WouldBlock);
                    }
                }
            }
        }
    }
    /// will busywait block on [send_nonblocking] untill it returns [WriteStatus::Completed]
    #[inline(always)]
    fn send_busywait(&mut self, msg: &M::SendT) -> Result<(), Error> {
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(()),
                WriteStatus::WouldBlock => continue,
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
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error>;
}
