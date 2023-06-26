use crate::prelude::*;
use bytes::{Buf, Bytes, BytesMut};
use framing::prelude::*;

// use crate::model::clt_heartbeat::ClientHeartbeat;
pub struct SoupBinTcp4FrameHandler;

impl FrameHandler for SoupBinTcp4FrameHandler {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        // ensures there is at least 2 bytes to represet packet_length
        if bytes.len() < 2 {
            return None;
        }

        // access packet length with out advancing the cursor, below is a take of the bytes::Buf::get_u16() method
        let packet_length = {
            const SIZE: usize = std::mem::size_of::<u16>();
            // try to convert directly from the bytes
            // this Option<ret> trick is to avoid keeping a borrow on self
            // when advance() is called (mut borrow) and to call bytes() only once
            let ret = bytes
                .chunk()
                .get(..SIZE)
                .map(|src| unsafe { u16::from_be_bytes(*(src as *const _ as *const [_; SIZE])) });

            if let Some(ret) = ret {
                ret
            } else {
                // if not we copy the bytes in a temp buffer then convert
                let mut buf = [0_u8; SIZE];
                let packet_length = &bytes[..SIZE];
                buf[0] = packet_length[0];
                buf[1] = packet_length[1];
                u16::from_be_bytes(buf)
            }
        };

        // ensure that there is a full frame available in the buffer
        let frame_length = (packet_length + 2) as usize;
        if bytes.len() < frame_length {
            return None;
        } else {
            let frame = bytes.split_to(frame_length);
            return Some(frame.freeze());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use byteserde::prelude::*;
    use log::info;

    use crate::{model::soup_bin::SoupBin, unittest::setup};

    #[test]
    fn test_soupbin_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<{ 13 * 11 }>::default();
        let msg_inp = SequencedData::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        for _ in 0..10 {
            let _ = msg_inp.byte_serialize_stack(&mut ser).unwrap();
        }

        println!("ser: {:#x}", ser);
        let mut buf = BytesMut::with_capacity(1024);

        buf.put_slice(ser.as_slice());

        // let mut buf = Bytes::from(bytes);
        loop {
            let frame = SoupBinTcp4FrameHandler::get_frame(&mut buf);
            match frame {
                Some(frame) => {
                    println!("frame: {:?}", frame);
                    let des = &mut ByteDeserializer::new(frame.chunk());
                    let x = SequencedData::byte_deserialize(des);
                    println!("{:?}", x);
                }
                None => break,
            }
        }
        println!("buf: {:?}", buf);
    }

    #[test]
    fn test_soup_bin_admin() {
        setup::log::configure();
        const CAP: usize = 1024;
        let mut ser = ByteSerializerStack::<CAP>::default();
        let msg_inp = vec![
            SoupBin::LoginRequest(LoginRequest::default()),
            SoupBin::LoginAccepted(LoginAccepted::default()),
        ];
        for m in msg_inp.iter() {
            let _ = ser.serialize(m).unwrap();
        }
        let mut bytes = BytesMut::with_capacity(CAP);
        bytes.put_slice(ser.as_slice());
        info!("ser: {:#x}", ser);
        info!("bytes: {:?}", bytes);
        loop {
            let frame = SoupBinTcp4FrameHandler::get_frame(&mut bytes);
            match frame {
                Some(frame) => {
                    let des = &mut ByteDeserializer::new(frame.chunk());
                    let msg = SoupBin::byte_deserialize(des).unwrap();
                    info!("msg: {:?}", msg);
                }
                None => break,
            }
        }
    }
}
