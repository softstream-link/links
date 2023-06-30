use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;

#[derive(
    ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
)]
#[byteserde(peek(0, 1))]
pub enum Ouch5 {
    #[byteserde(eq(PacketTypeEnterOrder::as_slice()))]
    EntOrd(EnterOrder),
    #[byteserde(eq(PacketTypeReplaceOrder::as_slice()))]
    RepOrd(ReplaceOrder),
    #[byteserde(eq(PacketTypeCancelOrder::as_slice()))]
    CanOrd(CancelOrder),
    #[byteserde(eq(PacketTypeModifyOrder::as_slice()))]
    ModOrd(ModifyOrder),
    #[byteserde(eq(PacketTypeAccountQueryRequest::as_slice()))]
    AccQryReq(AccountQueryRequest),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;

    #[test]
    fn test_ouch5() {
        setup::log::configure();

        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp = vec![
            Ouch5::EntOrd(EnterOrder::default()),
            Ouch5::RepOrd(ReplaceOrder::default()),
            Ouch5::CanOrd(CancelOrder::default()),
            Ouch5::ModOrd(ModifyOrder::default()),
            Ouch5::AccQryReq(AccountQueryRequest::default()),
        ];
        let _ = msg_inp.clone(); // to ensure clone is propagated to all Ouch5 variants

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out = vec![];
        while !des.is_empty() {
            let msg: Ouch5 = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
