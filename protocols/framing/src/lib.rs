pub mod prelude;

use bytes::{Bytes, BytesMut};

pub trait FrameHandler {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}
