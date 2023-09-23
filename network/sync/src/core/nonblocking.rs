use std::{
    io::Error,
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackRecvSend, Messenger};

use crate::prelude_nonblocking::Clt;

// ---- Acceptor ----

#[derive(Debug, PartialEq)]
pub enum PoolAcceptStatus {
    Accepted,
    WouldBlock,
}
impl PoolAcceptStatus {
    pub fn unwrap_accepted(self) -> () {
        match self {
            PoolAcceptStatus::Accepted => (),
            PoolAcceptStatus::WouldBlock => panic!("PoolAcceptStatus::WouldBlock"),
        }
    }
    pub fn is_accepted(&self) -> bool {
        match self {
            PoolAcceptStatus::Accepted => true,
            PoolAcceptStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        !self.is_accepted()
    }
}
pub trait PoolAcceptCltNonBlocking<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>:
    AcceptCltNonBlocking<M, C, MAX_MSG_SIZE>
{
    fn pool_accept_nonblocking(&mut self) -> Result<PoolAcceptStatus, Error>;
    fn pool_accept_busywait_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<PoolAcceptStatus, Error> {
        use PoolAcceptStatus::{Accepted, WouldBlock};
        let start = Instant::now();
        loop {
            match self.pool_accept_nonblocking()? {
                Accepted => return Ok(Accepted),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    fn pool_accept_busywait(&mut self) -> Result<(), Error> {
        use PoolAcceptStatus::{Accepted, WouldBlock};
        loop {
            match self.pool_accept_nonblocking()? {
                Accepted => return Ok(()),
                WouldBlock => continue,
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AcceptStatus<T> {
    Accepted(T),
    WouldBlock,
}
impl<T> AcceptStatus<T> {
    pub fn unwrap_accepted(self) -> T {
        match self {
            AcceptStatus::Accepted(t) => t,
            AcceptStatus::WouldBlock => panic!("AcceptStatus::WouldBlock"),
        }
    }
    pub fn is_accepted(&self) -> bool {
        match self {
            AcceptStatus::Accepted(_) => true,
            AcceptStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        !self.is_accepted()
    }
}
pub trait AcceptCltNonBlocking<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    fn accept_nonblocking(&self) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error>;

    fn accept_busywait_timeout(
        &self,
        timeout: Duration,
    ) -> Result<AcceptStatus<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        let start = Instant::now();
        loop {
            match self.accept_nonblocking()? {
                Accepted(clt) => return Ok(Accepted(clt)),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }

    fn accept_busywait(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        loop {
            match self.accept_nonblocking()? {
                Accepted(clt) => return Ok(clt),
                WouldBlock => continue,
            }
        }
    }
}

// ---- Recver ----

/// Represents the state of a non-blocking read operation
///
/// # Variants
/// * [RecvStatus::Completed(Some(T))] - indiates that read was successfull and `T` contains the value read
/// * [RecvStatus::Completed(None)] - indicates that connectioon was closed by the peer cleanly and all data was read
/// * [RecvStatus::WouldBlock] - indicates that no data was read and the caller should try again
#[derive(Debug, PartialEq)]
pub enum RecvStatus<T> {
    Completed(Option<T>),
    WouldBlock,
}

impl<T> RecvStatus<T> {
    /// Will panic if the variant is [RecvStatus::WouldBlock], otherwise unwraps into [`Option<T>`] from [RecvStatus::Completed(`Option<T>`)]
    pub fn unwrap_completed(self) -> Option<T> {
        match self {
            RecvStatus::Completed(o) => o,
            RecvStatus::WouldBlock => panic!("ReadStatus::WouldBlock"),
        }
    }
    pub fn is_completed(&self) -> bool {
        match self {
            RecvStatus::Completed(_) => true,
            RecvStatus::WouldBlock => false,
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
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<M::RecvT>, Error>;

    /// Will call [Self::recv_nonblocking] untill it returns [RecvStatus::Completed] or [RecvStatus::WouldBlock] after the timeout.
    fn recv_busywait_timeout(&mut self, timeout: Duration) -> Result<RecvStatus<M::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            match self.recv_nonblocking()? {
                Completed(o) => return Ok(Completed(o)),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Will busywait block on [Self::recv_nonblocking] untill it returns [RecvStatus::Completed]
    fn recv_busywait(&mut self) -> Result<Option<M::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        loop {
            match self.recv_nonblocking()? {
                Completed(o) => return Ok(o),
                WouldBlock => continue,
            }
        }
    }
}

// ---- Sender ----

/// Represents the state of the write operation
///
/// # Variants
///    * [SendStatus::Completed] - indicates that all bytes were written to the underlying stream
///    * [SendStatus::WouldBlock] - indicates that zero bytes were written to the underlying stream
#[derive(Debug, PartialEq)]
pub enum SendStatus {
    Completed,
    WouldBlock,
}

pub trait SendMsgNonBlocking<M: Messenger> {
    /// The call will internally serialize the msg and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some bytes from serialized message
    /// into the stream but the write was only partial then the call will buzy wait until all of
    /// remaining bytes were written before returning [SendStatus::Completed].
    /// [SendStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    fn send_nonblocking(&mut self, msg: &mut M::SendT) -> Result<SendStatus, Error>;

    /// Will call [Self::send_nonblocking] untill it returns [SendStatus::Completed] or [SendStatus::WouldBlock] after the timeoutok,
    #[inline(always)]
    fn send_busywait_timeout(
        &mut self,
        msg: &mut M::SendT,
        timeout: Duration,
    ) -> Result<SendStatus, Error> {
        let start = Instant::now();
        loop {
            match self.send_nonblocking(msg)? {
                SendStatus::Completed => return Ok(SendStatus::Completed),
                SendStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(SendStatus::WouldBlock);
                    }
                }
            }
        }
    }
    /// Will call [Self::send_nonblocking] untill it returns [SendStatus::Completed]
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut M::SendT) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        loop {
            match self.send_nonblocking(msg)? {
                Completed => return Ok(Completed),
                WouldBlock => continue,
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
