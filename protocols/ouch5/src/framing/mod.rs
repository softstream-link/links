#[cfg(test)]
mod test {
    use bytes::{BufMut, BytesMut};
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use log::info;
    use soupbintcp4::prelude::*;

    use crate::{
        model::ouch5::Ouch5,
        prelude::{EnterOrder, ReplaceOrder},
        unittest::setup,
    };
    use framing::FrameHandler;

    #[test]
    fn test_ouch5_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp = vec![
            SoupBin::CltHBeat(CltHeartbeat::default()),
            // SequencedData::new(Ouch5::EntOrd(EnterOrder::default())),
            // Ouch5::RepOrd(ReplaceOrder::default()),
        ];

        for msg in msg_inp {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(&SequencedData::new(msg));
        }

        println!("ser: {:#x}", ser);
        let mut buf = BytesMut::with_capacity(1024);

        buf.put_slice(ser.as_slice());

        let mut msg_out = vec![];
        while let Some(frame) = SoupBinTcp4FrameHandler::get_frame(&mut buf) {
            info!("frame:\n{}", to_hex_pretty(&frame[..]));
            let des = &mut ByteDeserializerSlice::new(&frame[..]);
            let msg: SoupBin<Ouch5> = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
