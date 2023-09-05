use std::error::Error;

use super::MessengerNew;



/// Represents the state of a non-blocking read operation
///
/// # Variants
///     * Completed(Some(T)) - indiates that read was successfull and `T` contains the value read
///     * Completed(None) - indicates that connectioon was closed by the peer cleanly and all data was read
///     * NotReady - indicates that no data was read and the caller should try again
#[derive(Debug)]
pub enum ReadStatus<T> {
    Completed(Option<T>),
    NotReady,
}

pub trait RecvMsgNonBlocking<M: MessengerNew>{
    /// Each call to this function
    fn recv(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>>;
}

/// Represents the state of the write operation
///
/// # Variants
///    * Completed - indicates that all bytes were written to the underlying stream
///    * NotReady - indicates that zero bytes were written to the underlying stream
#[derive(Debug)]
pub enum WriteStatus {
    Completed,
    NotReady,
}

pub trait SendMsgNonBlocking<M: MessengerNew>{
    /// If there was a successfull attempt to write any bytes from serialized message 
    /// into the stream but the write was only partial then the call shall buzy wait until all 
    /// remaining bytes were written before returning [WriteStatus::Completed]
    /// [WriteStatus::NotReady] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    fn send(&mut self, msg: &M::SendT) -> Result<WriteStatus, Box<dyn Error>>;
}
