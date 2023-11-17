pub mod framer;
pub mod messenger;

use std::{
    fmt::{Debug, Display},
    io::Error,
    time::{Duration, Instant},
};

use links_core::{prelude::Messenger, core::conid::ConId};

// ---- Acceptor ----

/// Represents the state of a non-blocking accept operation on a pool
///
/// # Variants
///  * [PoolAcceptStatus::Accepted] - indicates that accept was successful
///  * [PoolAcceptStatus::WouldBlock] - indicates that no connection was accepted
#[derive(Debug, PartialEq)]
pub enum PoolAcceptStatus {
    Accepted,
    WouldBlock,
}
impl PoolAcceptStatus {
    /// Unwraps to [()] if the variant is [PoolAcceptStatus::Accepted], otherwise panics
    pub fn unwrap(self) {
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
pub trait PoolAcceptCltNonBlocking {
    fn pool_accept(&mut self) -> Result<PoolAcceptStatus, Error>;
    /// Will call [Self::pool_accept] until it returns [PoolAcceptStatus::Accepted] or [PoolAcceptStatus::WouldBlock] after the timeout.
    fn pool_accept_busywait_timeout(&mut self, timeout: Duration) -> Result<PoolAcceptStatus, Error> {
        use PoolAcceptStatus::{Accepted, WouldBlock};
        let start = Instant::now();
        loop {
            match self.pool_accept()? {
                Accepted => return Ok(Accepted),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Will call [Self::pool_accept] until it returns [PoolAcceptStatus::Accepted]
    fn pool_accept_busywait(&mut self) -> Result<(), Error> {
        use PoolAcceptStatus::{Accepted, WouldBlock};
        loop {
            match self.pool_accept()? {
                Accepted => return Ok(()),
                WouldBlock => continue,
            }
        }
    }
}

/// Represents the state of a non-blocking accept operation
/// # Variants
/// * [AcceptStatus::Accepted(T)] - indicates that accept was successful and `T` contains the value accepted
/// * [AcceptStatus::WouldBlock] - indicates that no connection was accepted and the caller should try again
#[derive(Debug, PartialEq)]
pub enum AcceptStatus<T> {
    Accepted(T),
    WouldBlock,
}
impl<T> AcceptStatus<T> {
    /// Unwraps into [AcceptedStatus::Accepted(T)] if the variant is [AcceptStatus::Accepted], otherwise panics
    pub fn unwrap(self) -> T {
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
pub trait AcceptNonBlocking<T> {
    fn accept(&self) -> Result<AcceptStatus<T>, Error>;

    fn accept_busywait_timeout(&self, timeout: Duration) -> Result<AcceptStatus<T>, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        let start = Instant::now();
        loop {
            match self.accept()? {
                Accepted(t) => return Ok(Accepted(t)),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }

    fn accept_busywait(&self) -> Result<T, Error> {
        use AcceptStatus::{Accepted, WouldBlock};
        loop {
            match self.accept()? {
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
/// * [RecvStatus::Completed(Some(T))] - indicates that read was successful and `T` contains the value read
/// * [RecvStatus::Completed(None)] - indicates that connection was closed by the peer cleanly and all data was read
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
    /// Will panic if the variant is [RecvStatus::WouldBlock] or [RecvStatus::Completed(None)],  otherwise unwraps into `T` from [RecvStatus::Completed(Some(T))]
    pub fn unwrap(self) -> T {
        match self {
            RecvStatus::Completed(Some(t)) => t,
            RecvStatus::Completed(None) => panic!("ReadStatus::Completed(None)"),
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

pub trait RecvNonBlocking<M: Messenger> {
    /// Will attempt to read a message from the stream. Each call to this method will
    /// attempt to read data from the stream via system call and if sufficient number of bytes were read to
    /// make a single frame it will attempt to deserialize it into a message and return it
    fn recv(&mut self) -> Result<RecvStatus<M::RecvT>, Error>;

    /// Will call [Self::recv] until it returns [RecvStatus::Completed] or [RecvStatus::WouldBlock] after the timeout.
    fn recv_busywait_timeout(&mut self, timeout: Duration) -> Result<RecvStatus<M::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            match self.recv()? {
                Completed(o) => return Ok(Completed(o)),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Will busywait block on [Self::recv] until it returns [RecvStatus::Completed]
    fn recv_busywait(&mut self) -> Result<Option<M::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        loop {
            match self.recv()? {
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
impl SendStatus {
    /// Will panic if the variant is [SendStatus::WouldBlock], otherwise unwraps into [()] from [SendStatus::Completed]
    #[inline(always)]
    pub fn unwrap(self) {
        match self {
            SendStatus::Completed => {}
            SendStatus::WouldBlock => panic!("SendStatus::WouldBlock"),
        }
    }
    pub fn is_completed(&self) -> bool {
        match self {
            SendStatus::Completed => true,
            SendStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        !self.is_completed()
    }
}

pub trait SendNonBlocking<M: Messenger> {
    /// The call will internally serialize the msg and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some bytes from serialized message
    /// into the stream but the write was only partial then the call will busy wait until all of
    /// remaining bytes were written before returning [SendStatus::Completed].
    /// [SendStatus::WouldBlock] is returned only if the attempt did not write any bytes to the stream
    /// after the first attempt
    fn send(&mut self, msg: &mut M::SendT) -> Result<SendStatus, Error>;

    /// Will call [Self::send] until it returns [SendStatus::Completed] or [SendStatus::WouldBlock] after the timeout,
    /// while propagating all errors from [Self::send]
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut M::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            match self.send(msg)? {
                Completed => return Ok(Completed),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Will call [Self::send] until it returns [SendStatus::Completed]
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut M::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        loop {
            match self.send(msg)? {
                Completed => return Ok(()),
                WouldBlock => continue,
            }
        }
    }
}

pub trait SendNonBlockingNonMut<M: Messenger> {
    /// The call will internally serialize the msg and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some bytes from serialized message
    /// into the stream but the write was only partial then the call will busy wait until all of
    /// remaining bytes were written before returning [SendStatus::Completed].
    /// [SendStatus::WouldBlock] is returned only if the attempt did not write any bytes to the stream
    /// after the first attempt
    fn send(&mut self, msg: &<M as Messenger>::SendT) -> Result<SendStatus, Error>;

    /// Will call [Self::send] until it returns [SendStatus::Completed] or [SendStatus::WouldBlock] after the timeout,
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &<M as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        let start = Instant::now();
        loop {
            match self.send(msg)? {
                SendStatus::Completed => return Ok(SendStatus::Completed),
                SendStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(SendStatus::WouldBlock);
                    }
                }
            }
        }
    }
    /// Will call [Self::send] until it returns [SendStatus::Completed]
    #[inline(always)]
    fn send_busywait(&mut self, msg: &<M as Messenger>::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        loop {
            match self.send(msg)? {
                Completed => return Ok(()),
                WouldBlock => continue,
            }
        }
    }
}

// ---- Pool ----

#[derive(Debug)]
pub enum PollEventStatus {
    Completed,
    WouldBlock,
    Terminate,
}

pub trait PollRecv: Display + Send + 'static {
    fn source(&mut self) -> Box<&mut dyn mio::event::Source>;
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error>;
    fn con_id(&self) -> &ConId;
}

pub trait PollAccept<R: PollRecv>: PollRecv {
    fn poll_accept(&mut self) -> Result<AcceptStatus<R>, Error>;
    fn con_id(&self) -> &ConId;
}
