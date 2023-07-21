pub mod callbacks;
pub mod prelude;

#[cfg(test)]
pub mod unittest;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ConId {
    Clt(String),
    Svc(String),
}

pub trait Framer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

#[rustfmt::skip]
pub trait Messenger: Debug + Clone + Send + Sync + 'static {
    type Message: ByteDeserializeSlice<Self::Message> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
}

pub trait ProtocolHandler: Messenger + Framer + Send + Sync + 'static {}
