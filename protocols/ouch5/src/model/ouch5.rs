use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum Ouch5Inb {
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
/// Both [ReplaceOrder] & [OrderReplaced] are serialized as b'U' hence it is impossible to distinguish deserializetion type unless they are in two different enums.
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum Ouch5Oub {
    #[byteserde(eq(PacketTypeSystemEvent::as_slice()))]
    SysEvt(SystemEvent),
    #[byteserde(eq(PacketTypeOrderAccepted::as_slice()))]
    OrdAccptd(OrderAccepted),
    #[byteserde(eq(PacketTypeOrderReplaced::as_slice()))]
    OrdReplcd(OrderReplaced),
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
        let enter_ord = EnterOrder::default();
        let replace_ord = ReplaceOrder::from(&enter_ord);
        let cancel_ord = CancelOrder::from(&enter_ord);
        let ord_accepted = OrderAccepted::from(&enter_ord);
        let ord_replaced = OrderReplaced::from((&enter_ord, &replace_ord));
        let msg_inp_inb = vec![
            Ouch5Inb::EntOrd(enter_ord),
            Ouch5Inb::RepOrd(replace_ord),
            Ouch5Inb::CanOrd(cancel_ord),
            Ouch5Inb::ModOrd(ModifyOrder::default()),
            Ouch5Inb::AccQryReq(AccountQueryRequest::default()),
        ];
        let msg_inp_oub = vec![
            Ouch5Oub::SysEvt(SystemEvent::default()),
            Ouch5Oub::OrdAccptd(ord_accepted),
            Ouch5Oub::OrdReplcd(ord_replaced),
        ];
        let _ = msg_inp_inb.clone(); // to ensure clone is propagated to all Ouch5 variants
        let _ = msg_inp_oub.clone(); // to ensure clone is propagated to all Ouch5 variants

        for msg in msg_inp_inb.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        for msg in msg_inp_oub.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out_inb = vec![];
        let mut msg_out_oub = vec![];
        for _ in msg_inp_inb.iter() {
            let msg: Ouch5Inb = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out_inb.push(msg);
        }
        for _ in msg_inp_oub.iter() {
            let msg: Ouch5Oub = des.deserialize().unwrap();
            info!("msg_out: {:?}", msg);
            msg_out_oub.push(msg);
        }
        assert_eq!(msg_inp_inb, msg_out_inb);
        assert_eq!(msg_inp_oub, msg_out_oub);
        assert!(des.is_empty());
    }
}
