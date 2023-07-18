pub mod prelude;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

pub trait Framer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

// pub trait Message: ByteSerializeStack + Debug + Send + Sync + 'static {}
pub trait MessageHandler: Debug + Send + Sync + 'static {
    type Item: ByteDeserializeSlice<Self::Item> + ByteSerializeStack + Debug + Send + Sync + 'static;
    type FrameHandler: Framer + Debug + Send + Sync + 'static;

    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        Self::FrameHandler::get_frame(bytes)
    }

    #[inline(always)]
    fn into_msg(frame: Bytes) -> byteserde::prelude::Result<Self::Item> {
        from_slice::<Self::Item>(&frame[..])
    }
    #[inline(always)]
    fn from_msg<const STACK_SIZE: usize>(
        msg: &Self::Item,
    ) -> byteserde::prelude::Result<([u8; STACK_SIZE], usize)> {
        to_bytes_stack(msg)
    }
}

pub trait Messenger: Send + Sync + 'static {
    #[rustfmt::skip]
    type Message: ByteDeserializeSlice<Self::Message> + ByteSerializeStack + Debug + Send + Sync + 'static;
}


pub trait Callback: Messenger + Send + Sync + 'static {
    fn on_recv(&self, msg: Self::Message);
    // fn on_send(&self, msg: &mut Self::Message);
}


pub trait MessageFramer: Messenger + Framer + Send + Sync + 'static {}
