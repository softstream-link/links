use std::{fmt::Debug, io::Error};

use super::framer::Framer;
/// Trait defining `serialize` & `deserialize` methods for `Send` & `Recv` types
pub trait Messenger: Framer + Debug + Send + Sync + 'static {
    type SendT: Debug;
    type RecvT: Debug;

    /// Serializes application message of type [`Self::SendT`] into an array of size `MAX_MSG_SIZE` and return it along with the number 
    /// of bytes written as a tuple.
    /// 
    /// # Important
    /// * to avoid a copy of this array on the stack during function call remember to `#[inline]` implementation of this function
    fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), Error>;

    /// Deserializes application message from a byte slice and returns a concrete message of type [`Self::RecvT`].
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error>;
}
