pub mod conid;
pub mod counters;

use std::{error::Error, fmt::Debug};

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

// use crate::prelude::*;

/// Provides a function that is meant to determine when enough bytes are available to make up a single complete message/frame.
pub trait Framer {
    /// The implementation of this function should use protocol specific logic to determine when enough bytes are available
    /// and return the Some(Bytes) or None per below
    /// ```
    /// // if required_frame_len = frame_len {
    /// //     let frame = bytes.split_to(required_frame_len);
    /// //     Some(frame.freeze())
    /// // } else {
    /// //     None
    /// // }
    /// ```
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

/// Provides a two types that a peer in the connection can send or recv if the message types are the same
/// in both direction, just set to that same type in the implementation
#[rustfmt::skip]
pub trait Messenger: Debug+Send+Sync+'static {
    type SendT: ByteDeserializeSlice<Self::SendT>+ByteSerializeStack+Debug+Clone+PartialEq+Send+Sync+'static;
    type RecvT: ByteDeserializeSlice<Self::RecvT>+ByteSerializeStack+Debug+Clone+PartialEq+Send+Sync+'static;
}

// TODO rename to Messenger or add to prelude
pub trait MessengerNew: Framer {
    // const MAX_MESSAGE_SIZE_ASSOSIATED: usize = 128;
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    // fn serialize(msg: &mut Self::SendT) -> Result<([u8; Self::MAX_MESSAGE_SIZE_ASSOSIATED], usize), Box<dyn Error>>;
    fn serialize<const MAX_MESSAGE_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MESSAGE_SIZE], usize), Box<dyn Error>>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Box<dyn Error>>;
}
