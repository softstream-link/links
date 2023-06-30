#[cfg(test)]
mod test {
    use bytes::{BufMut, BytesMut};
    use byteserde::prelude::*;
    use soupbintcp4::prelude::*;

    use crate::{unittest::setup, prelude::{EnterOrder, ReplaceOrder}, model::ouch5::Ouch5};
    use framing::FrameHandler;

    #[test]
    fn test_ouch5_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        // let msg_inp = SequencedDataVec::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let msg_inp =vec![
            Ouch5::EntOrd(EnterOrder::default()),
            // Ouch5::RepOrd(ReplaceOrder::default()),
        ];

        for msg in msg_inp {
            let _ = ser.serialize(&SequencedData::new(msg));
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
        let mut msg_out: Vec<Ouch5> = vec![];
        loop {
            let frame = SoupBinTcp4FrameHandler::get_frame(&mut buf);
            match frame {
                Some(frame) => {
                    println!("frame: {:?}", frame);
                    let des = &mut ByteDeserializerSlice::new(&frame[..]);
                    let header: SoupBin = des.deserialize().unwrap();
                    match header{
                        SoupBin::SData(sdata) => {
                            let des = &mut ByteDeserializerSlice::new(sdata.chunk());
                            let msg = Ouch5::byte_deserialize(des).unwrap();
                            println!("{:?}", msg);
                            msg_out.push(msg);
                        },
                        _ => {}
                    }
            },
                None => break,
            }
        }
        println!("buf: {:?}", buf);
    }
}
