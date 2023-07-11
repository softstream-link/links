pub mod prelude;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

pub trait FrameHandler : Debug {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

pub trait MessageHandler<const STACK_SIZE: usize> {
    type MSG: ByteDeserializeSlice<Self::MSG> + ByteSerializeStack;
    type FHANDLER: FrameHandler + Debug;

    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        Self::FHANDLER::get_frame(bytes)
    }

    #[inline(always)]
    fn into_msg(frame: Bytes) -> byteserde::prelude::Result<Self::MSG> {
        from_slice::<Self::MSG>(&frame[..])
    }
    #[inline(always)]
    fn from_msg(msg: &Self::MSG) -> byteserde::prelude::Result<([u8; STACK_SIZE], usize)> {
        to_bytes_stack(msg)
    }
}
