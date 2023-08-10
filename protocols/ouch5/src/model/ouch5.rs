use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use links_soupbintcp_async::prelude::MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;

use crate::prelude::*;

use super::outbound::order_aiq_canceled::OrderAiqCanceled;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum Ouch5CltMsg {
    #[byteserde(eq(PacketTypeEnterOrder::as_slice()))]
    Enter(EnterOrder),
    #[byteserde(eq(PacketTypeReplaceOrder::as_slice()))]
    Replace(ReplaceOrder),
    #[byteserde(eq(PacketTypeCancelOrder::as_slice()))]
    Cancel(CancelOrder),
    #[byteserde(eq(PacketTypeModifyOrder::as_slice()))]
    Modify(ModifyOrder),
    #[byteserde(eq(PacketTypeAccountQueryRequest::as_slice()))]
    AccQry(AccountQueryRequest),
}

const MAX_FRAME_SIZE_SVC_MSG: usize = 72; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH5_SVC_MSG: usize =
    MAX_FRAME_SIZE_SVC_MSG + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;

const MAX_FRAME_SIZE_CLT_MSG: usize = 51; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH5_CLT_MSG: usize =
    MAX_FRAME_SIZE_CLT_MSG + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
/// Both [ReplaceOrder] & [OrderReplaced] are serialized as b'U' hence it is impossible to distinguish deserialization type unless they are in two different enums.
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum Ouch5SvcMsg {
    #[byteserde(eq(PacketTypeOrderAccepted::as_slice()))]
    Accepted(OrderAccepted),
    #[byteserde(eq(PacketTypeOrderExecuted::as_slice()))]
    Executed(OrderExecuted),
    #[byteserde(eq(PacketTypeOrderReplaced::as_slice()))]
    Replaced(OrderReplaced),
    #[byteserde(eq(PacketTypeOrderCanceled::as_slice()))]
    Canceled(OrderCanceled),
    #[byteserde(eq(PacketTypeOrderRejected::as_slice()))]
    Rejected(OrderRejected),
    #[byteserde(eq(PacketTypeOrderModified::as_slice()))]
    Modified(OrderModified),
    #[byteserde(eq(PacketTypeOrderRestated::as_slice()))]
    Restated(OrderRestated),

    #[byteserde(eq(PacketTypeCancelPending::as_slice()))]
    CanPending(CancelPending),
    #[byteserde(eq(PacketTypeCancelReject::as_slice()))]
    CanReject(CancelReject),
    #[byteserde(eq(PacketTypeOrderAiqCanceled::as_slice()))]
    AiqCanceled(OrderAiqCanceled),

    #[byteserde(eq(PacketTypeBrokenTrade::as_slice()))]
    BrokenTrade(BrokenTrade),    
    #[byteserde(eq(PacketTypePriorityUpdate::as_slice()))]
    PrioUpdate(PriorityUpdate),
    #[byteserde(eq(PacketTypeAccountQueryResponse::as_slice()))]
    AccQryRes(AccountQueryResponse),
    #[byteserde(eq(PacketTypeSystemEvent::as_slice()))]
    SysEvt(SystemEvent),
}

#[derive(Debug)]
pub enum Ouch5Msg {
    Clt(Ouch5CltMsg),
    Svc(Ouch5SvcMsg),
}

#[cfg(test)]
mod test {

    use crate::{
        model::ouch5::{MAX_FRAME_SIZE_CLT_MSG, MAX_FRAME_SIZE_SVC_MSG},
        prelude::*,
    };
    use byteserde::prelude::*;
    use links_testing::unittest::setup;
    use log::info;

    // TODO max message length needed to optimize stack serialization assume 512 bytes for now
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

        let msg_inp = vec![
            Ouch5Msg::Clt(Ouch5CltMsg::Enter(enter_ord)),
            Ouch5Msg::Clt(Ouch5CltMsg::Replace(replace_ord)),
            Ouch5Msg::Clt(Ouch5CltMsg::Cancel(cancel_ord)),
            Ouch5Msg::Clt(Ouch5CltMsg::Modify(ModifyOrder::default())),
            Ouch5Msg::Clt(Ouch5CltMsg::AccQry(AccountQueryRequest::default())),
            
            Ouch5Msg::Svc(Ouch5SvcMsg::SysEvt(SystemEvent::default())),
            Ouch5Msg::Svc(Ouch5SvcMsg::Accepted(ord_accepted)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Replaced(ord_replaced)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Canceled(ord_canceled)),
            Ouch5Msg::Svc(Ouch5SvcMsg::AiqCanceled(ord_aqi_canceled)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Executed(ord_executed)),
            Ouch5Msg::Svc(Ouch5SvcMsg::BrokenTrade(brkn_trade)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Rejected(ord_rejected)),
            Ouch5Msg::Svc(Ouch5SvcMsg::CanPending(can_pending)),
            Ouch5Msg::Svc(Ouch5SvcMsg::CanReject(can_reject)),
            Ouch5Msg::Svc(Ouch5SvcMsg::PrioUpdate(prio_update)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Modified(ord_modified)),
            Ouch5Msg::Svc(Ouch5SvcMsg::Restated(ord_rstd)),
            Ouch5Msg::Svc(Ouch5SvcMsg::AccQryRes(AccountQueryResponse::default())),
        ];
        let mut ser = ByteSerializerStack::<1024>::default();
        for ouch5 in msg_inp.iter() {
            match ouch5 {
                Ouch5Msg::Clt(msg_inp_inb) => {
                    info!("msg_inp_inb: {:?}", msg_inp_inb);
                    let _ = ser.serialize(msg_inp_inb).unwrap();
                }
                Ouch5Msg::Svc(msg_inp_oub) => {
                    info!("msg_inp_oub: {:?}", msg_inp_oub);
                    let _ = ser.serialize(msg_inp_oub).unwrap();
                }
            }
        }
        let mut des = ByteDeserializerSlice::new(ser.as_slice());

        for ouch5 in msg_inp.iter() {
            match ouch5 {
                Ouch5Msg::Clt(msg_inp_inb) => {
                    let msg_out_inb = des.deserialize::<Ouch5CltMsg>().unwrap();
                    info!("msg_out_inb: {:?}", msg_out_inb);
                    assert_eq!(msg_inp_inb, &msg_out_inb);
                }
                Ouch5Msg::Svc(msg_inp_oub) => {
                    let msg_out_oub = des.deserialize::<Ouch5SvcMsg>().unwrap();
                    info!("msg_out_oub: {:?}", msg_out_oub);
                    assert_eq!(msg_inp_oub, &msg_out_oub);
                }
            }
        }
        assert!(des.is_empty());
    }

    #[test]
    fn test_ouch5_max_size() {
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
        let inb = vec![
            Ouch5CltMsg::Enter(enter_ord),
            Ouch5CltMsg::Replace(replace_ord),
            Ouch5CltMsg::Cancel(cancel_ord),
            Ouch5CltMsg::Modify(ModifyOrder::default()),
            Ouch5CltMsg::AccQry(AccountQueryRequest::default()),
        ];
        let oub = vec![
            Ouch5SvcMsg::SysEvt(SystemEvent::default()),
            Ouch5SvcMsg::Accepted(ord_accepted),
            Ouch5SvcMsg::Replaced(ord_replaced),
            Ouch5SvcMsg::Canceled(ord_canceled),
            Ouch5SvcMsg::AiqCanceled(ord_aqi_canceled),
            Ouch5SvcMsg::Executed(ord_executed),
            Ouch5SvcMsg::BrokenTrade(brkn_trade),
            Ouch5SvcMsg::Rejected(ord_rejected),
            Ouch5SvcMsg::CanPending(can_pending),
            Ouch5SvcMsg::CanReject(can_reject),
            Ouch5SvcMsg::PrioUpdate(prio_update),
            Ouch5SvcMsg::Modified(ord_modified),
            Ouch5SvcMsg::Restated(ord_rstd),
            Ouch5SvcMsg::AccQryRes(AccountQueryResponse::default()),
        ];

        let inb = inb
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        // for (len, msg) in inb.iter() {
        //     info!("len: {:>3}, msg: Ouch5Inb::{:?}", len,  msg);
        // }
        let max_frame_size_clt = inb.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_clt: {}", max_frame_size_clt);
        assert_eq!(max_frame_size_clt, MAX_FRAME_SIZE_CLT_MSG);

        let oub = oub
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        // for (len, msg) in oub.iter() {
        //     info!("len: {:>3}, msg: Ouch5Oub::{:?}", len, msg);
        // }
        let max_frame_size_svc = oub.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_svc: {}", max_frame_size_svc);
        assert_eq!(max_frame_size_svc, MAX_FRAME_SIZE_SVC_MSG);
    }
}
