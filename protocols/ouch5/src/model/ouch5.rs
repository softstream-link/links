use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;

use super::outbound::order_aiq_canceled::OrderAiqCanceled;

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
    #[byteserde(eq(PacketTypeOrderCanceled::as_slice()))]
    OrdCancld(OrderCanceled),
    #[byteserde(eq(PacketTypeOrderAiqCanceled::as_slice()))]
    OrdAiqCancld(OrderAiqCanceled),
    #[byteserde(eq(PacketTypeOrderExecuted::as_slice()))]
    OrdExecd(OrderExecuted),
    #[byteserde(eq(PacketTypeBrokenTrade::as_slice()))]
    BrknTrd(BrokenTrade),
    #[byteserde(eq(PacketTypeOrderRejected::as_slice()))]
    OrdRjctd(OrderRejected),
    #[byteserde(eq(PacketTypeCancelPending::as_slice()))]
    CanPend(CancelPending),
    #[byteserde(eq(PacketTypeCancelReject::as_slice()))]
    CanRej(CancelReject),
    #[byteserde(eq(PacketTypePriorityUpdate::as_slice()))]
    PrioUpdt(PriorityUpdate),
    #[byteserde(eq(PacketTypeOrderModified::as_slice()))]
    OrdMod(OrderModified),
    #[byteserde(eq(PacketTypeOrderRestated::as_slice()))]
    OrdRstd(OrderRestated),
    #[byteserde(eq(PacketTypeAccountQueryResponse::as_slice()))]
    AccQryRes(AccountQueryResponse),
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
        let ord_canceled = OrderCanceled::from((&enter_ord, &cancel_ord));
        let ord_aqi_canceled = OrderAiqCanceled::from(&enter_ord);
        let ord_executed = OrderExecuted::from(&enter_ord);
        let brkn_trade = BrokenTrade::from(&enter_ord);
        let ord_rejected = OrderRejected::from((&enter_ord, RejectReason::halted()));
        let can_pending = CancelPending::from(&enter_ord);
        let can_reject = CancelReject::from(&enter_ord);
        let prio_update = PriorityUpdate::from((&enter_ord, OrderReferenceNumber::default()));
        let ord_modified = OrderModified::from((&enter_ord, Side::buy()));
        let ord_rstd = OrderRestated::from((&enter_ord, RestatedReason::refresh_of_display()));

        let msg_inp_inb: Vec<Ouch5Inb> = vec![
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
            Ouch5Oub::OrdCancld(ord_canceled),
            Ouch5Oub::OrdAiqCancld(ord_aqi_canceled),
            Ouch5Oub::OrdExecd(ord_executed),
            Ouch5Oub::BrknTrd(brkn_trade),
            Ouch5Oub::OrdRjctd(ord_rejected),
            Ouch5Oub::CanPend(can_pending),
            Ouch5Oub::CanRej(can_reject),
            Ouch5Oub::PrioUpdt(prio_update),
            Ouch5Oub::OrdMod(ord_modified),
            Ouch5Oub::OrdRstd(ord_rstd),
            Ouch5Oub::AccQryRes(AccountQueryResponse::default()),
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
