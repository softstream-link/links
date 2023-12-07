pub mod framer;
pub mod messenger;
pub mod protocol;

use links_core::{core::conid::ConnectionId, prelude::Messenger};
use mio::{Interest, Registry, Token};
use std::{
    fmt::{Debug, Display},
    io::Error,
    time::{Duration, Instant},
};

// ---- Acceptor ----

/// Represents the state of a non-blocking accept operation on a pool
///
/// # Variants
///  * [PoolAcceptStatus::Accepted] - indicates that accept was successful
///  * [PoolAcceptStatus::WouldBlock] - indicates that no connection was accepted
#[derive(Debug, PartialEq)]
pub enum PoolAcceptStatus {
    Accepted,
    Rejected,
    WouldBlock,
}
impl PoolAcceptStatus {
    /// Unwraps to [()] if the variant is [PoolAcceptStatus::Accepted], otherwise panics
    #[track_caller]
    pub fn unwrap_accepted(self) {
        match self {
            PoolAcceptStatus::Accepted => (),
            PoolAcceptStatus::Rejected => panic!("PoolAcceptStatus::Rejected"),
            PoolAcceptStatus::WouldBlock => panic!("PoolAcceptStatus::WouldBlock"),
        }
    }
    #[track_caller]
    pub fn unwrap_rejected(self) {
        match self {
            PoolAcceptStatus::Accepted => panic!("PoolAcceptStatus::Accepted"),
            PoolAcceptStatus::Rejected => (),
            PoolAcceptStatus::WouldBlock => panic!("PoolAcceptStatus::WouldBlock"),
        }
    }
    pub fn is_accepted(&self) -> bool {
        match self {
            PoolAcceptStatus::Accepted => true,
            PoolAcceptStatus::Rejected => false,
            PoolAcceptStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        match self {
            PoolAcceptStatus::Accepted => false,
            PoolAcceptStatus::Rejected => false,
            PoolAcceptStatus::WouldBlock => true,
        }
    }
    pub fn is_rejected(&self) -> bool {
        match self {
            PoolAcceptStatus::Accepted => false,
            PoolAcceptStatus::Rejected => true,
            PoolAcceptStatus::WouldBlock => false,
        }
    }
}
pub trait PoolSvcAcceptorOfCltNonBlocking {
    fn pool_accept(&mut self) -> Result<PoolAcceptStatus, Error>;
    /// Will call [Self::accept_into_pool] until it returns [PoolAcceptStatus::Accepted] or [PoolAcceptStatus::WouldBlock] after the timeout.
    fn accept_into_pool_busywait_timeout(&mut self, timeout: Duration) -> Result<PoolAcceptStatus, Error> {
        use PoolAcceptStatus::{Accepted, Rejected, WouldBlock};
        let start = Instant::now();
        loop {
            match self.pool_accept()? {
                Accepted => return Ok(Accepted),
                Rejected => return Ok(Rejected),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Will call [Self::accept_into_pool] until it returns [PoolAcceptStatus::Accepted]
    fn accept_into_pool_busywait(&mut self) -> Result<PoolAcceptStatus, Error> {
        use PoolAcceptStatus::{Accepted, Rejected, WouldBlock};
        loop {
            match self.pool_accept()? {
                Accepted => return Ok(Accepted),
                Rejected => return Ok(Rejected),
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
    Rejected,
    WouldBlock,
}
impl<T> AcceptStatus<T> {
    /// Unwraps into [AcceptedStatus::Accepted(T)] if the variant is [AcceptStatus::Accepted], otherwise panics
    #[track_caller]
    pub fn unwrap_accepted(self) -> T {
        match self {
            AcceptStatus::Accepted(t) => t,
            AcceptStatus::Rejected => panic!("AcceptStatus::Rejected"),
            AcceptStatus::WouldBlock => panic!("AcceptStatus::WouldBlock"),
        }
    }
    pub fn is_accepted(&self) -> bool {
        match self {
            AcceptStatus::Accepted(_) => true,
            AcceptStatus::Rejected => false,
            AcceptStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        match self {
            AcceptStatus::Accepted(_) => false,
            AcceptStatus::Rejected => false,
            AcceptStatus::WouldBlock => true,
        }
    }
    pub fn is_rejected(&self) -> bool {
        match self {
            AcceptStatus::Accepted(_) => false,
            AcceptStatus::Rejected => true,
            AcceptStatus::WouldBlock => false,
        }
    }
}
pub trait SvcAcceptorOfCltNonBlocking<T> {
    fn accept(&self) -> Result<AcceptStatus<T>, Error>;

    fn accept_busywait_timeout(&self, timeout: Duration) -> Result<AcceptStatus<T>, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        let start = Instant::now();
        loop {
            match self.accept()? {
                Accepted(t) => return Ok(Accepted(t)),
                Rejected => return Ok(Rejected),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }

    fn accept_busywait(&self) -> Result<T, Error> {
        use AcceptStatus::{Accepted, Rejected, WouldBlock};
        loop {
            match self.accept()? {
                Accepted(clt) => return Ok(clt),
                Rejected => continue,
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
    #[track_caller]
    pub fn unwrap_completed_none(self) {
        match self {
            RecvStatus::Completed(Some(_)) => panic!("ReadStatus::Completed(Some(_))"),
            RecvStatus::Completed(None) => (),
            RecvStatus::WouldBlock => panic!("ReadStatus::WouldBlock"),
        }
    }
    /// Will panic if the variant is [RecvStatus::WouldBlock] or [RecvStatus::Completed(None)],  otherwise unwraps into `T` from [RecvStatus::Completed(Some(T))]
    #[track_caller]
    pub fn unwrap_completed_some(self) -> T {
        match self {
            RecvStatus::Completed(Some(t)) => t,
            RecvStatus::Completed(None) => panic!("ReadStatus::Completed(None)"),
            RecvStatus::WouldBlock => panic!("ReadStatus::WouldBlock"),
        }
    }
    #[track_caller]
    pub fn unwrap_wouldblock(self) {
        match self {
            RecvStatus::Completed(_) => panic!("ReadStatus::Completed(_)"),
            RecvStatus::WouldBlock => {}
        }
    }
    pub fn is_completed(&self) -> bool {
        match self {
            RecvStatus::Completed(_) => true,
            RecvStatus::WouldBlock => false,
        }
    }
    pub fn is_completed_none(&self) -> bool {
        match self {
            RecvStatus::Completed(Some(_)) => false,
            RecvStatus::Completed(None) => true,
            RecvStatus::WouldBlock => false,
        }
    }
    pub fn is_completed_some(&self) -> bool {
        match self {
            RecvStatus::Completed(Some(_)) => true,
            RecvStatus::Completed(None) => false,
            RecvStatus::WouldBlock => false,
        }
    }
    pub fn is_wouldblock(&self) -> bool {
        match self {
            RecvStatus::Completed(_) => false,
            RecvStatus::WouldBlock => true,
        }
    }
}

pub trait RecvNonBlocking<M: Messenger>: Debug + Display {
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
    #[track_caller]
    pub fn unwrap_completed(self) {
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

pub trait SendNonBlocking<M: Messenger>: Debug + Display {
    /// The call will internally serialize the [Messenger::SendT] and attempt to write the resulting bytes into a stream.
    /// If there was a successfull attempt which wrote some, not all, bytes from serialized message
    /// into the stream and hence the write was only partial, the call will busy wait until all of
    /// remaining bytes are written before returning [SendStatus::Completed].
    /// [SendStatus::WouldBlock] is returned only if the attempt did not write any bytes to the stream
    /// after the first attempt
    fn send(&mut self, msg: &mut M::SendT) -> Result<SendStatus, Error>;

    /// Will call [Self::send] until it returns [SendStatus::Completed] or [SendStatus::WouldBlock] after the timeout,
    /// while propagating all errors from [Self::send]
    ///
    /// # Warning
    /// Consider overriding this default implementation if your [Self::send] implementation issues callback functions
    /// calls which must be called once and only once.
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
    ///
    /// # Warning
    /// Consider overriding this default implementation if your [Self::send] implementation issues callback functions
    /// calls which must be called once and only once.
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

/// A trait to be implemented for a type that can be registered with a [mio::Poll] instance, it helps to
/// abstract away details of [Token] generation registration and de-registration
/// The source will get automatically deregistered from the poll instance when [PollReadable::on_readable_event]
/// returns [Ok(PollEventStatus::Terminate)] or [Err(_)]
pub trait PollReadable: ConnectionId + Display + Send + 'static {
    /// this function exists as a hook in case you need to perform resource locking prior to registering the source
    /// 
    /// # Warning
    /// [PollReadable::source] usage will depend on your override implementation and may not used
    fn register(&mut self, registry: &Registry, token: Token, interests: Interest) -> Result<(), Error> {
        registry.register(*self.source(), token, interests)?;
        Ok(())
    }
    /// this function exists as a hook in case you need to perform resource locking prior to de-registering the source
    /// 
    /// # Warning 
    /// [PollReadable::source] usage will depend on your override implementation and may not used
    fn deregister(&mut self, registry: &Registry) -> Result<(), Error> {
        registry.deregister(*self.source())?;
        Ok(())
    }
    /// represents the source of the event, typically implementing this function is sufficient as [register] and [deregister] functions
    /// are implemented using it to get the source for the poll. However You can choose to override [register] and [deregister] functions
    /// when for example you require to lock a mutes to get access to the source in which case source function will not be used.
    fn source(&mut self) -> Box<&mut dyn mio::event::Source>;
    /// Will be called when OS signals that the source is readable
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error>;
}

/// A trait to be implemented for a type that can be registered with a [mio::Poll] instance, it helps to
/// abstract away details of [Token] generation registration and de-registration specifically monitoring
pub trait PollAccept<R: PollReadable>: PollReadable {
    fn poll_accept(&mut self) -> Result<AcceptStatus<R>, Error>;
}
