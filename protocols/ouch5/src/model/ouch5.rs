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

#[derive(Debug)]
pub enum Ouch5 {
    Inb(Ouch5Inb),
    Oub(Ouch5Oub),
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use byteserde::prelude::*;
    use crate::unittest::setup;
    use log::info;

    // TODO max message length needed to optimize stack serialization assuem 512 bytes for now
    #[test]
    fn test_ouch5() {
        setup::log::configure();

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

        let msg_inp: Vec<Ouch5> = vec![
            Ouch5::Inb(Ouch5Inb::EntOrd(enter_ord)),
            Ouch5::Inb(Ouch5Inb::RepOrd(replace_ord)),
            Ouch5::Inb(Ouch5Inb::CanOrd(cancel_ord)),
            Ouch5::Inb(Ouch5Inb::ModOrd(ModifyOrder::default())),
            Ouch5::Inb(Ouch5Inb::AccQryReq(AccountQueryRequest::default())),
            Ouch5::Oub(Ouch5Oub::SysEvt(SystemEvent::default())),
            Ouch5::Oub(Ouch5Oub::OrdAccptd(ord_accepted)),
            Ouch5::Oub(Ouch5Oub::OrdReplcd(ord_replaced)),
            Ouch5::Oub(Ouch5Oub::OrdCancld(ord_canceled)),
            Ouch5::Oub(Ouch5Oub::OrdAiqCancld(ord_aqi_canceled)),
            Ouch5::Oub(Ouch5Oub::OrdExecd(ord_executed)),
            Ouch5::Oub(Ouch5Oub::BrknTrd(brkn_trade)),
            Ouch5::Oub(Ouch5Oub::OrdRjctd(ord_rejected)),
            Ouch5::Oub(Ouch5Oub::CanPend(can_pending)),
            Ouch5::Oub(Ouch5Oub::CanRej(can_reject)),
            Ouch5::Oub(Ouch5Oub::PrioUpdt(prio_update)),
            Ouch5::Oub(Ouch5Oub::OrdMod(ord_modified)),
            Ouch5::Oub(Ouch5Oub::OrdRstd(ord_rstd)),
            Ouch5::Oub(Ouch5Oub::AccQryRes(AccountQueryResponse::default())),
        ];
        let mut ser = ByteSerializerStack::<1024>::default();
        for ouch5 in msg_inp.iter() {
            match ouch5 {
                Ouch5::Inb(msg_inp_inb) => {
                    info!("msg_inp_inb: {:?}", msg_inp_inb);
                    let _ = ser.serialize(msg_inp_inb).unwrap();
                }
                Ouch5::Oub(msg_inp_oub) => {
                    info!("msg_inp_oub: {:?}", msg_inp_oub);
                    let _ = ser.serialize(msg_inp_oub).unwrap();
                }
            }
        }
        let mut des = ByteDeserializerSlice::new(ser.as_slice());

        for ouch5 in msg_inp.iter() {
            match ouch5 {
                Ouch5::Inb(msg_inp_inb) => {
                    let msg_out_inb = des.deserialize::<Ouch5Inb>().unwrap();
                    info!("msg_out_inb: {:?}", msg_out_inb);
                    assert_eq!(msg_inp_inb, &msg_out_inb);
                }
                Ouch5::Oub(msg_inp_oub) => {
                    let msg_out_oub = des.deserialize::<Ouch5Oub>().unwrap();
                    info!("msg_out_oub: {:?}", msg_out_oub);
                    assert_eq!(msg_inp_oub, &msg_out_oub);
                }
            }
        }
        assert!(des.is_empty());
    }
}
