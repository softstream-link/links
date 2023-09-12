pub mod blocking;
use std::{fmt::Debug, io::Error};

use super::Framer;

// TODO rename to Messenger or add to prelude

// TODO remove is MessengerNew works with out clone - Why clone, because when injected into the CallbackRecv Trait and this trait imp is used on the Svc, it must be able to clone it for each accepted connection
pub trait MessengerNew: Framer+Debug {
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    fn serialize<const MAX_MESSAGE_SIZE: usize>(
        msg: &Self::SendT,
    ) -> Result<([u8; MAX_MESSAGE_SIZE], usize), Error>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error>;
}
