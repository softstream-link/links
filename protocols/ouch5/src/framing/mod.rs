use bytes::{Buf, Bytes, BytesMut};
use framing::FrameHandler;
use soupbintcp4::prelude::SoupBinTcp4FrameHandler;
struct Ouch5FrameHandler;

impl FrameHandler for Ouch5FrameHandler {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinTcp4FrameHandler::get_frame(bytes)
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
        // let x = buf.freeze();
        
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
