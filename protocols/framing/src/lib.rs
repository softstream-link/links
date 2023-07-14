pub mod prelude;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

pub trait FrameHandler: Debug {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

pub trait Message: ByteSerializeStack + Debug + Send + Sync + 'static {}
pub trait MessageHandler: Debug + Send + Sync + 'static {
    type Item: ByteDeserializeSlice<Self::Item> + ByteSerializeStack + Debug + Send + Sync + 'static;
    type FrameHandler: FrameHandler + Debug + Send + Sync + 'static;

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
