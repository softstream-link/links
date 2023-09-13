use std::{fmt::Debug, io::Error};

use super::Framer;

pub trait Messenger: Framer+Debug {
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    fn serialize<const MAX_MESSAGE_SIZE: usize>(
        msg: &Self::SendT,
    ) -> Result<([u8; MAX_MESSAGE_SIZE], usize), Error>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error>;
}
