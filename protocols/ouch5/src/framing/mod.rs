use bytes::{Buf, Bytes, BytesMut};
use framing::FrameHandler;
struct Ouch5FrameHandler;

impl FrameHandler for Ouch5FrameHandler {
    fn get_frame(bytes: &mut BytesMut) -> Option<BytesMut> {
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
            return Some(frame);
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use byteserde::prelude::*;
    use soupbintcp4::prelude::*;

    use crate::{framing::Ouch5FrameHandler, unittest::setup};
    use framing::FrameHandler;

    #[test]
    fn test_ouch5_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<{ 13 * 11 }>::default();
        let msg_inp = SequencedData::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        for _ in 0..10 {
            let _ = msg_inp.byte_serialize_stack(&mut ser).unwrap();
        }
        // let bytes = ser.as_slice().to_owned().into_boxed_slice();
        // ser.as_slice().into
        // let bytes = Box::new(to_bytes_stack::<128, SequencedData>(&msg_inp).unwrap());
        // let bytes = to_serializer_stack::<128, SequencedData>(&msg_inp).unwrap().as_slice().to_owned().into_boxed_slice();
        // let x = bytes.to_owned().into_boxed_slice();
        println!("ser: {:#x}", ser);
        let mut buf = BytesMut::with_capacity(1024);
        
        buf.put_slice(ser.as_slice());
        let x = buf.freeze();
        
        // let mut buf = Bytes::from(bytes);
        loop {
            let frame = Ouch5FrameHandler::get_frame(&mut buf);
            match frame{
                Some(frame) => println!("frame: {:?}", frame), 
                None => break,
            }
        }
        println!("buf: {:?}", buf);


    }
}
