use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use links_soupbintcp_async::prelude::{
    SBCltMsg, SBMsg, SBSvcMsg, MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG,
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

pub const MAX_FRAME_SIZE_OUCH_SVC_PLD: usize = 72; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_SVC_MSG: usize =
    MAX_FRAME_SIZE_OUCH_SVC_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;

pub const MAX_FRAME_SIZE_OUCH_CLT_PLD: usize = 51; // TODO revise Options fields and remeasure
pub const MAX_FRAME_SIZE_OUCH_CLT_MSG: usize =
    MAX_FRAME_SIZE_OUCH_CLT_PLD + MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
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

pub type OuchCltMsg = SBCltMsg<OuchCltPld>;
pub type OuchSvcMsg = SBSvcMsg<OuchSvcPld>;

pub type OuchMsg = SBMsg<OuchCltPld, OuchSvcPld>; // TODO shoud this be this?

pub struct OuchCltMsgEnv;
impl OuchCltMsgEnv {
    #[inline]
    pub fn clt(payload: OuchCltPld) -> OuchMsg {
        OuchMsg::Clt(SBCltMsg::udata(payload))
    }
    #[inline]
    pub fn ent(payload: EnterOrder) -> OuchMsg {
        Self::clt(OuchCltPld::Enter(payload))
    }
    #[inline]
    pub fn rep(payload: ReplaceOrder) -> OuchMsg {
        Self::clt(OuchCltPld::Replace(payload))
    }
    #[inline]
    pub fn can(payload: CancelOrder) -> OuchMsg {
        Self::clt(OuchCltPld::Cancel(payload))
    }
    #[inline]
    pub fn mof(payload: ModifyOrder) -> OuchMsg {
        Self::clt(OuchCltPld::Modify(payload))
    }
    #[inline]
    pub fn accqry(payload: AccountQueryRequest) -> OuchMsg {
        Self::clt(OuchCltPld::AccQry(payload))
    }
}

pub struct OuchSvcMsgEnv;
impl OuchSvcMsgEnv {
    #[inline]
    pub fn svc(payload: OuchSvcPld) -> OuchMsg {
        OuchMsg::Svc(SBSvcMsg::udata(payload))
    }
    #[inline]
    pub fn acced(payload: OrderAccepted) -> OuchMsg {
        Self::svc(OuchSvcPld::Accepted(payload))
    }
    #[inline]
    pub fn exeed(payload: OrderExecuted) -> OuchMsg {
        Self::svc(OuchSvcPld::Executed(payload))
    }
    #[inline]
    pub fn reped(payload: OrderReplaced) -> OuchMsg {
        Self::svc(OuchSvcPld::Replaced(payload))
    }
    #[inline]
    pub fn caned(payload: OrderCanceled) -> OuchMsg {
        Self::svc(OuchSvcPld::Canceled(payload))
    }
    #[inline]
    pub fn rejed(payload: OrderRejected) -> OuchMsg {
        Self::svc(OuchSvcPld::Rejected(payload))
    }
    #[inline]
    pub fn mofed(payload: OrderModified) -> OuchMsg {
        Self::svc(OuchSvcPld::Modified(payload))
    }
    #[inline]
    pub fn resed(payload: OrderRestated) -> OuchMsg {
        Self::svc(OuchSvcPld::Restated(payload))
    }
    #[inline]
    pub fn caning(payload: CancelPending) -> OuchMsg {
        Self::svc(OuchSvcPld::CanPending(payload))
    }
    #[inline]
    pub fn canrej(payload: CancelReject) -> OuchMsg {
        Self::svc(OuchSvcPld::CanReject(payload))
    }
    #[inline]
    pub fn aiqcaned(payload: OrderAiqCanceled) -> OuchMsg {
        Self::svc(OuchSvcPld::AiqCanceled(payload))
    }
    #[inline]
    pub fn bkntrd(payload: BrokenTrade) -> OuchMsg {
        Self::svc(OuchSvcPld::BrokenTrade(payload))
    }
    #[inline]
    pub fn priupd(payload: PriorityUpdate) -> OuchMsg {
        Self::svc(OuchSvcPld::PrioUpdate(payload))
    }
    #[inline]
    pub fn accqry(payload: AccountQueryResponse) -> OuchMsg {
        Self::svc(OuchSvcPld::AccQryRes(payload))
    }
    #[inline]
    pub fn sysevt(payload: SystemEvent) -> OuchMsg {
        Self::svc(OuchSvcPld::SysEvt(payload))
    }

}

#[cfg(test)]
mod test {

    use crate::{
        model::ouch::{MAX_FRAME_SIZE_OUCH_CLT_PLD, MAX_FRAME_SIZE_OUCH_SVC_PLD},
        prelude::*,
    };
    use byteserde::prelude::*;
    use links_soupbintcp_async::prelude::SBMsg;
    use links_testing::unittest::setup;
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
            OuchCltMsgEnv::ent(enter_ord),
            OuchCltMsgEnv::rep(replace_ord),
            OuchCltMsgEnv::can(cancel_ord),
            OuchCltMsgEnv::mof(ModifyOrder::default()),
            OuchCltMsgEnv::accqry(AccountQueryRequest::default()),

            OuchSvcMsgEnv::acced(ord_accepted),
            OuchSvcMsgEnv::exeed(ord_executed),
            OuchSvcMsgEnv::reped(ord_replaced),
            OuchSvcMsgEnv::caned(ord_canceled),
            OuchSvcMsgEnv::rejed(ord_rejected),
            OuchSvcMsgEnv::mofed(ord_modified),
            OuchSvcMsgEnv::resed(ord_rstd),
            OuchSvcMsgEnv::caning(can_pending),
            OuchSvcMsgEnv::canrej(can_reject),
            OuchSvcMsgEnv::aiqcaned(ord_aqi_canceled),
            OuchSvcMsgEnv::bkntrd(brkn_trade),
            OuchSvcMsgEnv::priupd(prio_update),
            OuchSvcMsgEnv::accqry(AccountQueryResponse::default()),
            OuchSvcMsgEnv::sysevt(SystemEvent::default()),
        ];
        let mut ser = ByteSerializerStack::<1024>::default();
        for msg in msg_inp.iter() {
            match msg {
                SBMsg::Clt(msg_inp_inb) => {
                    info!("msg_inp_inb: {:?}", msg_inp_inb);
                    let _ = ser.serialize(msg_inp_inb).unwrap();
                }
                SBMsg::Svc(msg_inp_oub) => {
                    info!("msg_inp_oub: {:?}", msg_inp_oub);
                    let _ = ser.serialize(msg_inp_oub).unwrap();
                }
            }
        }
        let mut des = ByteDeserializerSlice::new(ser.as_slice());

        for ouch in msg_inp.iter() {
            match ouch {
                SBMsg::Clt(msg_inp_inb) => {
                    let msg_out_inb = des.deserialize::<OuchCltMsg>().unwrap();
                    info!("msg_out_inb: {:?}", msg_out_inb);
                    assert_eq!(msg_inp_inb, &msg_out_inb);
                }
                SBMsg::Svc(msg_inp_oub) => {
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

        let inb = inb
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        // for (len, msg) in inb.iter() {
        //     info!("len: {:>3}, msg: Ouch5Inb::{:?}", len,  msg);
        // }
        let max_frame_size_clt = inb.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_clt: {}", max_frame_size_clt);
        assert_eq!(max_frame_size_clt, MAX_FRAME_SIZE_OUCH_CLT_PLD);

        let oub = oub
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        // for (len, msg) in oub.iter() {
        //     info!("len: {:>3}, msg: Ouch5Oub::{:?}", len, msg);
        // }
        let max_frame_size_svc = oub.iter().map(|(len, _)| *len).max().unwrap();
        info!("max_frame_size_svc: {}", max_frame_size_svc);
        assert_eq!(max_frame_size_svc, MAX_FRAME_SIZE_OUCH_SVC_PLD);
    }
}
