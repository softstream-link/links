pub mod nonblocking;
pub mod blocking;
use std::{fmt::Debug, error::Error};

use super::Framer;


// TODO rename to Messenger or add to prelude
pub trait MessengerNew: Framer {
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    fn serialize<const MAX_MESSAGE_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MESSAGE_SIZE], usize), Box<dyn Error>>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Box<dyn Error>>;
}

