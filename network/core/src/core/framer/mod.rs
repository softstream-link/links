use bytes::{Bytes, BytesMut};

/// Provides a function that is meant to determine when enough bytes are available to make up a single complete message/frame.
pub trait Framer {
    /// The implementation of this function should use protocol specific logic to determine when enough bytes are available
    /// and return the Some(Bytes) or None per below
    /// ```no_run
    /// // if bytes.len() >= FRAME_SIZE {
    /// //     let frame = bytes.split_to(FRAME_SIZE);
    /// //     Some(frame.freeze())
    /// // } else {
    /// //     None
    /// // }
    /// ```
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

pub struct FixedSizeFramer<const FRAME_SIZE: usize>;
impl<const FRAME_SIZE: usize> Framer for FixedSizeFramer<FRAME_SIZE> {
    /// Provides a default implementation of [Framer] that simply returns the first [FixedSizeFramer<FRAME_SIZE>] bytes as a frame.
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        if bytes.len() >= FRAME_SIZE {
            let frame = bytes.split_to(FRAME_SIZE);
            Some(frame.freeze())
        } else {
            None
        }
    }
}
