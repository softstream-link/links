pub mod blocking;
use std::{fmt::Debug, error::Error};

use super::Framer;


// TODO rename to Messenger or add to prelude

// Why clone, because when injected into the CallbackRecv Trait and this trait imp is used on the Svc, it must be able to clone it for each accepted connection
pub trait MessengerNew: Framer + Debug + Clone{
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    fn serialize<const MAX_MESSAGE_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MESSAGE_SIZE], usize), Box<dyn Error>>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Box<dyn Error>>;
}

