use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use links_soupbintcp_async::prelude::{
    SBCltMsg, SBSvcMsg, MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG,
};

use crate::prelude::*;

use super::svc::order_aiq_canceled::OrderAiqCanceled;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum OuchCltPld {
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

const MAX_FRAME_SIZE_OUCH5_SVC_PLD: usize = 72; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_SVC_MSG: usize =
    MAX_FRAME_SIZE_OUCH5_SVC_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;

const MAX_FRAME_SIZE_OUCH5_CLT_PLD: usize = 51; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_CLT_MSG: usize =
    MAX_FRAME_SIZE_OUCH5_CLT_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
/// Both [ReplaceOrder] & [OrderReplaced] are serialized as b'U' hence it is impossible to distinguish deserialization type unless they are in two different enums.
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum OuchSvcPld {
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

#[derive(Debug, Clone)]
pub enum OuchMsg {
    Clt(SBCltMsg<OuchCltPld>),
    Svc(SBSvcMsg<OuchSvcPld>),
}
impl OuchMsg {
    pub fn clt(payload: OuchCltPld) -> Self {
        Self::Clt(SBCltMsg::udata(payload))
    }
    pub fn enter_order(payload: EnterOrder) -> Self {
        Self::Clt(SBCltMsg::udata(OuchCltPld::Enter(payload)))
    }
    pub fn svc(payload: OuchSvcPld) -> Self {
        Self::Svc(SBSvcMsg::udata(payload))
    }
}
impl From<SBCltMsg<OuchCltPld>> for OuchMsg {
    fn from(msg: SBCltMsg<OuchCltPld>) -> Self {
        Self::Clt(msg)
    }
}
impl From<SBSvcMsg<OuchSvcPld>> for OuchMsg {
    fn from(msg: SBSvcMsg<OuchSvcPld>) -> Self {
        Self::Svc(msg)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        model::ouch::{MAX_FRAME_SIZE_OUCH5_CLT_PLD, MAX_FRAME_SIZE_OUCH5_SVC_PLD},
        prelude::*,
    };
    use byteserde::prelude::*;
    use links_soupbintcp_async::prelude::{SBCltMsg, SBSvcMsg};
    use links_testing::unittest::setup;
    use log::info;

    #[test]
    fn test_from() {
        setup::log::configure();
        // let enter_order = Ouch5Msg::enter_order(EnterOrder::default());
        let enter_order = SBCltMsg::udata(OuchCltPld::Enter(EnterOrder::default()));
        info!("enter_order: {:?}", enter_order);
        let ouch_msg = OuchMsg::from(enter_order);
        info!("ouch_msg: {:?}", ouch_msg);

    }
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
            OuchMsg::clt(OuchCltPld::Enter(enter_ord)),
            OuchMsg::clt(OuchCltPld::Replace(replace_ord)),
            OuchMsg::clt(OuchCltPld::Cancel(cancel_ord)),
            OuchMsg::clt(OuchCltPld::Modify(ModifyOrder::default())),
            OuchMsg::clt(OuchCltPld::AccQry(AccountQueryRequest::default())),
            OuchMsg::svc(OuchSvcPld::SysEvt(SystemEvent::default())),
            OuchMsg::svc(OuchSvcPld::Accepted(ord_accepted)),
            OuchMsg::svc(OuchSvcPld::Replaced(ord_replaced)),
            OuchMsg::svc(OuchSvcPld::Canceled(ord_canceled)),
            OuchMsg::svc(OuchSvcPld::AiqCanceled(ord_aqi_canceled)),
            OuchMsg::svc(OuchSvcPld::Executed(ord_executed)),
            OuchMsg::svc(OuchSvcPld::BrokenTrade(brkn_trade)),
            OuchMsg::svc(OuchSvcPld::Rejected(ord_rejected)),
            OuchMsg::svc(OuchSvcPld::CanPending(can_pending)),
            OuchMsg::svc(OuchSvcPld::CanReject(can_reject)),
            OuchMsg::svc(OuchSvcPld::PrioUpdate(prio_update)),
            OuchMsg::svc(OuchSvcPld::Modified(ord_modified)),
            OuchMsg::svc(OuchSvcPld::Restated(ord_rstd)),
            OuchMsg::svc(OuchSvcPld::AccQryRes(AccountQueryResponse::default())),
        ];
        let mut ser = ByteSerializerStack::<1024>::default();
        for ouch5 in msg_inp.iter() {
            match ouch5 {
                OuchMsg::Clt(msg_inp_inb) => {
                    info!("msg_inp_inb: {:?}", msg_inp_inb);
                    let _ = ser.serialize(msg_inp_inb).unwrap();
                }
                OuchMsg::Svc(msg_inp_oub) => {
                    info!("msg_inp_oub: {:?}", msg_inp_oub);
                    let _ = ser.serialize(msg_inp_oub).unwrap();
                }
            }
        }
        let mut des = ByteDeserializerSlice::new(ser.as_slice());

        for ouch5 in msg_inp.iter() {
            match ouch5 {
                OuchMsg::Clt(msg_inp_inb) => {
                    let msg_out_inb = des.deserialize::<SBCltMsg<OuchCltPld>>().unwrap();
                    info!("msg_out_inb: {:?}", msg_out_inb);
                    assert_eq!(msg_inp_inb, &msg_out_inb);
                }
                OuchMsg::Svc(msg_inp_oub) => {
                    let msg_out_oub = des.deserialize::<SBSvcMsg<OuchSvcPld>>().unwrap();
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
            OuchCltPld::Enter(enter_ord),
            OuchCltPld::Replace(replace_ord),
            OuchCltPld::Cancel(cancel_ord),
            OuchCltPld::Modify(ModifyOrder::default()),
            OuchCltPld::AccQry(AccountQueryRequest::default()),
        ];
        let oub = vec![
            OuchSvcPld::SysEvt(SystemEvent::default()),
            OuchSvcPld::Accepted(ord_accepted),
            OuchSvcPld::Replaced(ord_replaced),
            OuchSvcPld::Canceled(ord_canceled),
            OuchSvcPld::AiqCanceled(ord_aqi_canceled),
            OuchSvcPld::Executed(ord_executed),
            OuchSvcPld::BrokenTrade(brkn_trade),
            OuchSvcPld::Rejected(ord_rejected),
            OuchSvcPld::CanPending(can_pending),
            OuchSvcPld::CanReject(can_reject),
            OuchSvcPld::PrioUpdate(prio_update),
            OuchSvcPld::Modified(ord_modified),
            OuchSvcPld::Restated(ord_rstd),
            OuchSvcPld::AccQryRes(AccountQueryResponse::default()),
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
        assert_eq!(max_frame_size_clt, MAX_FRAME_SIZE_OUCH5_CLT_PLD);

        let oub = oub
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        // for (len, msg) in oub.iter() {
        //     info!("len: {:>3}, msg: Ouch5Oub::{:?}", len, msg);
        // }
        let max_frame_size_svc = oub.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_svc: {}", max_frame_size_svc);
        assert_eq!(max_frame_size_svc, MAX_FRAME_SIZE_OUCH5_SVC_PLD);
    }
}
