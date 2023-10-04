use bytes::{Bytes, BytesMut};

/// Provides a function that is meant to determine when enough bytes are available to make up a single complete message/frame.
pub trait Framer {
    /// The implementation of this function should use protocol specific logic to determine when enough bytes are available
    /// to make up a single complete message/frame and return the [Some(usize)] frame length or [None] if not enough bytes are available.
    fn get_frame_length(bytes: &mut BytesMut) -> Option<usize>;

    /// Will return a frame as [Some(Bytes)] of length determined by the [Self::get_frame_length] function.
    #[inline(always)]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        let frame_length = Self::get_frame_length(bytes)?;
        if bytes.len() < frame_length {
            None
        } else {
            let frame = bytes.split_to(frame_length);
            Some(frame.freeze())
        }
    }
}

/// Will split the first `<const FRAME_SIZE: usize>` bytes from the [BytesMut] buffer and return them as a [Bytes] frame.
pub struct FixedSizeFramer<const FRAME_SIZE: usize>;
impl<const FRAME_SIZE: usize> Framer for FixedSizeFramer<FRAME_SIZE> {
    #[inline(always)]
    fn get_frame_length(bytes: &mut BytesMut) -> Option<usize> {
        if bytes.len() < FRAME_SIZE {
            None
        } else {
            Some(FRAME_SIZE)
        }
    }
}

pub struct PacketLengthU16Framer<
    const START_IDX: usize,
    const IS_BIG_ENDIAN: bool,
    const ADD_PACKET_LEN_TO_FRAME_SIZE: bool,
>;
impl<
        const START_IDX: usize,
        const IS_BIG_ENDIAN: bool,
        const ADD_PACKET_LEN_TO_FRAME_SIZE: bool,
    > PacketLengthU16Framer<START_IDX, IS_BIG_ENDIAN, ADD_PACKET_LEN_TO_FRAME_SIZE>
{
    #[inline(always)]
    pub fn packet_len(bytes: &mut BytesMut) -> Option<u16> {
        const LEN: usize = std::mem::size_of::<u16>();
        // ensures there is at least [0 1 2 START/3 4 END/5] LEN=2 for u16
        if bytes.len() < START_IDX + LEN {
            return None;
        }

        // access packet length with out advancing the cursor, below is a take of the bytes::Buf::get_u16() method
        let packet_length = {
            // try to convert directly from the bytes
            // this Option<ret> trick is to avoid keeping a borrow on self
            // when advance() is called (mut borrow) and to call bytes() only once
            let opt = bytes.get(START_IDX..LEN).map(|src| {
                if IS_BIG_ENDIAN {
                    unsafe { u16::from_be_bytes(*(src as *const _ as *const [_; LEN])) }
                } else {
                    unsafe { u16::from_le_bytes(*(src as *const _ as *const [_; LEN])) }
                }
            });

            if let Some(len) = opt {
                len
            } else {
                // if not we copy the bytes in a temp buffer then convert
                let mut buf = [0_u8; LEN];
                let packet_length = &bytes[START_IDX..LEN];
                buf[0] = packet_length[0];
                buf[1] = packet_length[1];
                if IS_BIG_ENDIAN {
                    u16::from_be_bytes(buf)
                } else {
                    u16::from_le_bytes(buf)
                }
            }
        };
        Some(packet_length)
    }
}
impl<
        const START_IDX: usize,
        const IS_BIG_ENDIAN: bool,
        const ADD_PACKET_LEN_TO_FRAME_SIZE: bool,
    > Framer for PacketLengthU16Framer<START_IDX, IS_BIG_ENDIAN, ADD_PACKET_LEN_TO_FRAME_SIZE>
{
    #[inline(always)]
    fn get_frame_length(bytes: &mut BytesMut) -> Option<usize> {
        let packet_length = Self::packet_len(bytes)?;

        let frame_length = {
            if ADD_PACKET_LEN_TO_FRAME_SIZE {
                packet_length as usize + START_IDX + std::mem::size_of::<u16>()
            } else {
                packet_length as usize
            }
        };
        if bytes.len() < frame_length {
            None
        } else {
            Some(frame_length)
        }
    }
}

#[cfg(test)]
mod test {

    use bytes::{BufMut, BytesMut};
    use log::info;

    use crate::{core::framer::PacketLengthU16Framer, unittest::setup};

    use super::{FixedSizeFramer, Framer};

    #[test]
    fn test_fixed_size_framer() {
        setup::log::configure();
        let mut buf = BytesMut::from(&b"12345"[..]);

        let frame = FixedSizeFramer::<2>::get_frame(&mut buf).unwrap();
        info!("frame: {:?}", frame);
        assert_eq!(frame, &b"12"[..]);

        let frame = FixedSizeFramer::<2>::get_frame(&mut buf).unwrap();
        info!("frame: {:?}", frame);
        assert_eq!(frame, &b"34"[..]);

        let frame = FixedSizeFramer::<2>::get_frame(&mut buf);
        info!("frame: {:?}", frame);
        assert!(frame.is_none());
    }

    #[test]
    fn test_packet_len_u16_framer() {
        setup::log::configure();

        const START: usize = 0;
        const ADD_PACKET_LEN_TO_FRAME_SIZE: bool = true;
        let test_lens = [0x0001_u16, 0x0100];
        let frame_lens_big = [Some(3_usize), None];
        let frame_lens_lit = [None, Some(3_usize)];

        for (idx, expected_packet_len) in test_lens.into_iter().enumerate() {
            info!("idx: ==== {:?} ==== BIG ENDIAN", idx);
            let mut bytes = BytesMut::from(expected_packet_len.to_be_bytes().as_slice());
            bytes.put_bytes(0, 1);

            let actual_packet_len =
                PacketLengthU16Framer::<START, true, ADD_PACKET_LEN_TO_FRAME_SIZE>::packet_len(
                    &mut bytes,
                )
                .unwrap();
            info!("bytes: {:x?}", &bytes[..]);
            info!(
                "expected_packet_len: {:x?} {:?}",
                expected_packet_len.to_be_bytes(),
                expected_packet_len,
            );
            info!(
                "actual_packet_len: {:x?} {:?}",
                actual_packet_len.to_be_bytes(),
                actual_packet_len,
            );

            assert_eq!(actual_packet_len, expected_packet_len);

            let frame_len =
                PacketLengthU16Framer::<START, true, ADD_PACKET_LEN_TO_FRAME_SIZE>::get_frame_length(
                    &mut bytes,
                )
                ;
            info!("frame_len: {:?}", frame_len);
            assert_eq!(frame_len, frame_lens_big[idx]);

            info!("idx: ==== {:?} ==== LIT ENDIAN", idx);

            let actual_packet_len =
                PacketLengthU16Framer::<START, false, ADD_PACKET_LEN_TO_FRAME_SIZE>::packet_len(
                    &mut bytes,
                )
                .unwrap();
            let expected_packet_len = u16::from_be_bytes(expected_packet_len.to_le_bytes()); // flip byte order to match Framer
            info!("bytes: {:x?}", &bytes[..]);
            info!(
                "expected_packet_len: {:x?} {:?}",
                expected_packet_len.to_be_bytes(),
                expected_packet_len,
            );
            info!(
                "actual_packet_len: {:x?} {:?}",
                actual_packet_len.to_be_bytes(),
                actual_packet_len,
            );
            assert_eq!(actual_packet_len, expected_packet_len);

            let frame_len =
            PacketLengthU16Framer::<START, false, ADD_PACKET_LEN_TO_FRAME_SIZE>::get_frame_length(
                &mut bytes,
            )
            ;
            info!("frame_len: {:?}", frame_len);
            assert_eq!(frame_len, frame_lens_lit[idx]);
        }
    }
}
