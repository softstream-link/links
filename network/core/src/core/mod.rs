pub mod conid;
pub mod counters;
pub mod macros;
pub mod framer;
pub mod messenger;
pub mod pool;
use std::fmt::Debug;

use byteserde::prelude::*;

// TODO remove after refactoring async
/// Provides a two types that a peer in the connection can send or recv if the message types are the same
/// in both direction, just set to that same type in the implementation
#[rustfmt::skip]
pub trait MessengerOld: Debug+Send+Sync+'static {
    type SendT: ByteDeserializeSlice<Self::SendT>+ByteSerializeStack+Debug+Clone+PartialEq+Send+Sync+'static;
    type RecvT: ByteDeserializeSlice<Self::RecvT>+ByteSerializeStack+Debug+Clone+PartialEq+Send+Sync+'static;
}
