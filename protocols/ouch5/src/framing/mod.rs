#[cfg(test)]
mod test {
    use bytes::{BufMut, BytesMut};
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use log::info;
    use soupbintcp4::prelude::*;

    use crate::{prelude::*, unittest::setup};
    use framing::FrameHandler;

    #[test]
    fn test_ouch5_inbound_frame_handler() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        type Ouch5 = SoupBinMsg<Ouch5Inb>;
        let msg_inp = vec![
            Ouch5::sdata(Ouch5Inb::EntOrd(EnterOrder::default())),
            Ouch5::sdata(Ouch5Inb::RepOrd(ReplaceOrder::from(&EnterOrder::default()))),
            Ouch5::sdata(Ouch5Inb::CanOrd(CancelOrder::from(&EnterOrder::default()))),
            Ouch5::sdata(Ouch5Inb::ModOrd(ModifyOrder::default())),
            Ouch5::sdata(Ouch5Inb::AccQryReq(AccountQueryRequest::default())),
        ];
        

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg);
        }

        println!("ser: {:#x}", ser);
        let mut buf = BytesMut::with_capacity(1024);

        buf.put_slice(ser.as_slice());

        let mut msg_out = vec![];
        while let Some(frame) = SoupBinFrame::get_frame(&mut buf) {
            info!("frame:\n{}", to_hex_pretty(&frame[..]));
            let des = &mut ByteDeserializerSlice::new(&frame[..]);
            let msg: SoupBinMsg<Ouch5Inb> = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
