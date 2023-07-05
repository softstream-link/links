#[cfg(test)]
mod test {
    use bytes::{BufMut, BytesMut};
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use log::info;
    use soupbintcp4::prelude::*;

    use crate::{prelude::*, unittest::setup};
    use framing::FrameHandler;

    #[test]
    fn test_ouch5_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp: Vec<SoupBin<Ouch5Inb>> = vec![
            SoupBin::CltHBeat(CltHeartbeat::default()),
            SoupBin::SvcHBeat(SvcHeartbeat::default()),
            SoupBin::SData(SequencedData::new(Ouch5Inb::EntOrd(EnterOrder::default()))),
            SoupBin::SData(SequencedData::new(Ouch5Inb::RepOrd(ReplaceOrder::from(&EnterOrder::default())))),
            SoupBin::SData(SequencedData::new(Ouch5Inb::CanOrd(CancelOrder::from(&EnterOrder::default())))),
            SoupBin::SData(SequencedData::new(Ouch5Inb::ModOrd(ModifyOrder::default()))),
            SoupBin::SData(SequencedData::new(Ouch5Inb::AccQryReq(AccountQueryRequest::default()))),
        ];

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg);
        }

        println!("ser: {:#x}", ser);
        let mut buf = BytesMut::with_capacity(1024);

        buf.put_slice(ser.as_slice());

        let mut msg_out = vec![];
        while let Some(frame) = SoupBinTcp4FrameHandler::get_frame(&mut buf) {
            info!("frame:\n{}", to_hex_pretty(&frame[..]));
            let des = &mut ByteDeserializerSlice::new(&frame[..]);
            let msg: SoupBin<Ouch5Inb> = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
