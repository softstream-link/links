use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use derive_more::TryInto;
use links_soupbintcp_async::prelude::{SBCltMsg, SBMsg, SBSvcMsg, MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG};

use crate::prelude::*;

use super::svc::order_aiq_canceled::OrderAiqCanceled;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, TryInto)]
#[try_into(owned, ref, ref_mut)]
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

pub const MAX_FRAME_SIZE_OUCH_SVC_PLD: usize = 72; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_SVC_MSG: usize = MAX_FRAME_SIZE_OUCH_SVC_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;

pub const MAX_FRAME_SIZE_OUCH_CLT_PLD: usize = 51; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_CLT_MSG: usize = MAX_FRAME_SIZE_OUCH_CLT_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
/// Both [ReplaceOrder] & [OrderReplaced] are serialized as b'U' hence it is impossible to distinguish deserialization type unless they are in two different enums.
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, TryInto)]
#[try_into(owned, ref, ref_mut)]
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

pub type OuchCltMsg = SBCltMsg<OuchCltPld>;
pub type OuchSvcMsg = SBSvcMsg<OuchSvcPld>;

pub type OuchMsg = SBMsg<OuchCltPld, OuchSvcPld>;

mod from_clt_pld {
    use super::*;
    impl From<EnterOrder> for OuchCltMsg {
        #[inline]
        fn from(payload: EnterOrder) -> Self {
            OuchCltMsg::udata(OuchCltPld::Enter(payload))
        }
    }
    impl From<EnterOrder> for OuchMsg {
        #[inline]
        fn from(payload: EnterOrder) -> Self {
            OuchMsg::Clt(payload.into())
        }
    }
    impl From<ReplaceOrder> for OuchCltMsg {
        #[inline]
        fn from(payload: ReplaceOrder) -> Self {
            OuchCltMsg::udata(OuchCltPld::Replace(payload))
        }
    }
    impl From<ReplaceOrder> for OuchMsg {
        #[inline]
        fn from(payload: ReplaceOrder) -> Self {
            OuchMsg::Clt(payload.into())
        }
    }
    impl From<CancelOrder> for OuchCltMsg {
        #[inline]
        fn from(payload: CancelOrder) -> Self {
            OuchCltMsg::udata(OuchCltPld::Cancel(payload))
        }
    }
    impl From<CancelOrder> for OuchMsg {
        #[inline]
        fn from(payload: CancelOrder) -> Self {
            OuchMsg::Clt(payload.into())
        }
    }
    impl From<ModifyOrder> for OuchCltMsg {
        #[inline]
        fn from(payload: ModifyOrder) -> Self {
            OuchCltMsg::udata(OuchCltPld::Modify(payload))
        }
    }
    impl From<ModifyOrder> for OuchMsg {
        #[inline]
        fn from(payload: ModifyOrder) -> Self {
            OuchMsg::Clt(payload.into())
        }
    }
    impl From<AccountQueryRequest> for OuchCltMsg {
        #[inline]
        fn from(payload: AccountQueryRequest) -> Self {
            OuchCltMsg::udata(OuchCltPld::AccQry(payload))
        }
    }
    impl From<AccountQueryRequest> for OuchMsg {
        #[inline]
        fn from(payload: AccountQueryRequest) -> Self {
            OuchMsg::Clt(payload.into())
        }
    }
}

mod from_svc_pld {
    use super::*;
    impl From<OrderAccepted> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderAccepted) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Accepted(payload))
        }
    }
    impl From<OrderAccepted> for OuchMsg {
        #[inline]
        fn from(payload: OrderAccepted) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderExecuted> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderExecuted) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Executed(payload))
        }
    }
    impl From<OrderExecuted> for OuchMsg {
        #[inline]
        fn from(payload: OrderExecuted) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderReplaced> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderReplaced) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Replaced(payload))
        }
    }
    impl From<OrderReplaced> for OuchMsg {
        #[inline]
        fn from(payload: OrderReplaced) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderCanceled> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderCanceled) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Canceled(payload))
        }
    }
    impl From<OrderCanceled> for OuchMsg {
        #[inline]
        fn from(payload: OrderCanceled) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderRejected> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderRejected) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Rejected(payload))
        }
    }
    impl From<OrderRejected> for OuchMsg {
        #[inline]
        fn from(payload: OrderRejected) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderModified> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderModified) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Modified(payload))
        }
    }
    impl From<OrderModified> for OuchMsg {
        #[inline]
        fn from(payload: OrderModified) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderRestated> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderRestated) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::Restated(payload))
        }
    }
    impl From<OrderRestated> for OuchMsg {
        #[inline]
        fn from(payload: OrderRestated) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<CancelPending> for OuchSvcMsg {
        #[inline]
        fn from(payload: CancelPending) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::CanPending(payload))
        }
    }
    impl From<CancelPending> for OuchMsg {
        #[inline]
        fn from(payload: CancelPending) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<CancelReject> for OuchSvcMsg {
        #[inline]
        fn from(payload: CancelReject) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::CanReject(payload))
        }
    }
    impl From<CancelReject> for OuchMsg {
        #[inline]
        fn from(payload: CancelReject) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<OrderAiqCanceled> for OuchSvcMsg {
        #[inline]
        fn from(payload: OrderAiqCanceled) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::AiqCanceled(payload))
        }
    }
    impl From<OrderAiqCanceled> for OuchMsg {
        #[inline]
        fn from(payload: OrderAiqCanceled) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<BrokenTrade> for OuchSvcMsg {
        #[inline]
        fn from(payload: BrokenTrade) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::BrokenTrade(payload))
        }
    }
    impl From<BrokenTrade> for OuchMsg {
        #[inline]
        fn from(payload: BrokenTrade) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<PriorityUpdate> for OuchSvcMsg {
        #[inline]
        fn from(payload: PriorityUpdate) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::PrioUpdate(payload))
        }
    }
    impl From<PriorityUpdate> for OuchMsg {
        #[inline]
        fn from(payload: PriorityUpdate) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<AccountQueryResponse> for OuchSvcMsg {
        #[inline]
        fn from(payload: AccountQueryResponse) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::AccQryRes(payload))
        }
    }
    impl From<AccountQueryResponse> for OuchMsg {
        #[inline]
        fn from(payload: AccountQueryResponse) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
    impl From<SystemEvent> for OuchSvcMsg {
        #[inline]
        fn from(payload: SystemEvent) -> Self {
            OuchSvcMsg::udata(OuchSvcPld::SysEvt(payload))
        }
    }
    impl From<SystemEvent> for OuchMsg {
        #[inline]
        fn from(payload: SystemEvent) -> Self {
            OuchMsg::Svc(payload.into())
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{
        model::ouch::{MAX_FRAME_SIZE_OUCH_CLT_PLD, MAX_FRAME_SIZE_OUCH_SVC_PLD},
        prelude::*,
    };
    use byteserde::prelude::*;
    use links_core::unittest::setup;
    use log::info;

    // TODO max message length needed to optimize stack serialization assume 512 bytes for now
    #[test]
    fn test_ouch_with_envelope_ser_des() {
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
            enter_ord.into(),
            replace_ord.into(),
            cancel_ord.into(),
            ModifyOrder::default().into(),
            AccountQueryRequest::default().into(),
            ord_accepted.into(),
            ord_executed.into(),
            ord_replaced.into(),
            ord_canceled.into(),
            ord_rejected.into(),
            ord_modified.into(),
            ord_rstd.into(),
            can_pending.into(),
            can_reject.into(),
            ord_aqi_canceled.into(),
            brkn_trade.into(),
            prio_update.into(),
            AccountQueryResponse::default().into(),
            SystemEvent::default().into(),
        ];
        let mut ser = ByteSerializerStack::<1024>::default();
        for msg in msg_inp.iter() {
            match msg {
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

        for ouch in msg_inp.iter() {
            match ouch {
                OuchMsg::Clt(msg_inp_inb) => {
                    let msg_out_inb = des.deserialize::<OuchCltMsg>().unwrap();
                    info!("msg_out_inb: {:?}", msg_out_inb);
                    assert_eq!(msg_inp_inb, &msg_out_inb);
                }
                OuchMsg::Svc(msg_inp_oub) => {
                    let msg_out_oub = des.deserialize::<OuchSvcMsg>().unwrap();
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

        let inb = inb.into_iter().map(|msg| (msg.byte_len(), msg)).collect::<Vec<_>>();
        // for (len, msg) in inb.iter() {
        //     info!("len: {:>3}, msg: Ouch5Inb::{:?}", len,  msg);
        // }
        let max_frame_size_clt = inb.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_clt: {}", max_frame_size_clt);
        assert_eq!(max_frame_size_clt, MAX_FRAME_SIZE_OUCH_CLT_PLD);

        let oub = oub.into_iter().map(|msg| (msg.byte_len(), msg)).collect::<Vec<_>>();
        // for (len, msg) in oub.iter() {
        //     info!("len: {:>3}, msg: Ouch5Oub::{:?}", len, msg);
        // }
        let max_frame_size_svc = oub.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_svc: {}", max_frame_size_svc);
        assert_eq!(max_frame_size_svc, MAX_FRAME_SIZE_OUCH_SVC_PLD);
    }
}
