use std::{fmt::Debug, io::Error};

use super::framer::Framer;

pub trait Messenger: Framer+Debug {
    type SendT: Debug+Clone+PartialEq;
    type RecvT: Debug+Clone+PartialEq;
    
    //  TODO explore how to return a &[u8] instead of a [u8; MAX_MESSAGE_SIZE] with self as an argument
    fn serialize<const MAX_MSG_SIZE: usize>(
        msg: &Self::SendT,
    ) -> Result<([u8; MAX_MSG_SIZE], usize), Error>;
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error>;
}