use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use derive_more::TryInto;

use crate::prelude::*;
use std::fmt;

use super::unsequenced_data::UPayload;

pub const MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG: usize = 54;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug, TryInto)]
#[byteserde(peek(2, 1))]
pub enum SBCltMsg<CltPayload: ByteSerializeStack + ByteDeserializeSlice<CltPayload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> {
    #[byteserde(eq(PacketTypeUnsequencedData::as_slice()))]
    U(UPayload::<CltPayload>),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    S(SPayload::<CltPayload>),
    #[byteserde(eq(PacketTypeCltHeartbeat::as_slice()))]
    HBeat(CltHeartbeat),
    #[byteserde(eq(PacketTypeDebug::as_slice()))]
    Dbg(crate::model::debug::Debug),
    #[byteserde(eq(PacketTypeLoginRequest::as_slice()))]
    Login(LoginRequest),
    #[byteserde(eq(PacketTypeLogoutRequest::as_slice()))]
    Logout(LogoutRequest),
}
#[rustfmt::skip]
impl<CltPayload: ByteSerializeStack + ByteDeserializeSlice<CltPayload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> SBCltMsg<CltPayload> {
    pub fn login(username: UserName, password: Password, session_id: SessionId, sequence_number: SequenceNumber, hbeat_timeout_ms: TimeoutMs) -> Self { 
        Self::Login( LoginRequest::new(username, password, session_id, sequence_number, hbeat_timeout_ms)) 
    }
    pub fn logout() -> Self { SBCltMsg::Logout(LogoutRequest::default()) }
    pub fn hbeat() -> Self { SBCltMsg::HBeat(CltHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { SBCltMsg::Dbg(Debug::new(text)) }
    pub fn sdata(payload: CltPayload) -> Self { SBCltMsg::S(SPayload::new(payload)) }
    pub fn udata(payload: CltPayload) -> Self { SBCltMsg::U(UPayload::new(payload)) }
}
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug, TryInto)]
#[byteserde(peek(2, 1))]
pub enum SBSvcMsg<SvcPayload: ByteSerializeStack + ByteDeserializeSlice<SvcPayload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug>{
    #[byteserde(eq(PacketTypeUnsequencedData::as_slice()))]
    U(UPayload::<SvcPayload>),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    S(SPayload::<SvcPayload>),
    #[byteserde(eq(PacketTypeSvcHeartbeat::as_slice()))]
    HBeat(SvcHeartbeat),
    #[byteserde(eq(PacketTypeDebug::as_slice()))]
    Dbg(crate::model::debug::Debug),
    #[byteserde(eq(PacketTypeEndOfSession::as_slice()))]
    End(EndOfSession),
    #[byteserde(eq(PacketTypeLoginAccepted::as_slice()))]
    LoginAcc(LoginAccepted),
    #[byteserde(eq(PacketTypeLoginRejected::as_slice()))]
    LoginRej(LoginRejected),
}
#[rustfmt::skip]
impl<SvcPayload: ByteSerializeStack + ByteDeserializeSlice<SvcPayload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> SBSvcMsg<SvcPayload> {
    pub fn end() -> Self { Self::End(EndOfSession::default()) }
    pub fn login_acc(session_id: SessionId, sequence_number: SequenceNumber) -> Self { Self::LoginAcc(LoginAccepted::new(session_id, sequence_number)) }
    pub fn login_rej_not_auth() -> Self { Self::LoginRej(LoginRejected::not_authorized()) }
    pub fn login_rej_ses_not_avail() -> Self { Self::LoginRej(LoginRejected::session_not_available()) }
    pub fn hbeat() -> Self { Self::HBeat(SvcHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { Self::Dbg(Debug::new(text)) }
    pub fn sdata(payload: SvcPayload) -> Self { Self::S(SPayload::new(payload)) }
    pub fn udata(payload: SvcPayload) -> Self { Self::U(UPayload::new(payload)) }
}

#[derive(Debug, Clone, PartialEq, TryInto)]
pub enum SBMsg<CltPayload, SvcPayload>
where
    CltPayload: ByteSerializeStack+ByteDeserializeSlice<CltPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
    SvcPayload: ByteSerializeStack+ByteDeserializeSlice<SvcPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
{
    Clt(SBCltMsg<CltPayload>),
    Svc(SBSvcMsg<SvcPayload>),
}
impl<CltPayload, SvcPayload> SBMsg<CltPayload, SvcPayload>
where
    CltPayload: ByteSerializeStack+ByteDeserializeSlice<CltPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
    SvcPayload: ByteSerializeStack+ByteDeserializeSlice<SvcPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
{
    pub fn unwrap_clt_u(&self) -> &CltPayload{
        match self {
            SBMsg::Clt(SBCltMsg::U(UPayload{body, ..})) => body,
            _ => panic!("SoupBinTcp message is not Clt and/or UPayload, instead it is: {:?}", self),
        }
    }
    pub fn unwrap_svc_u(&self) -> &SvcPayload{
        match self {
            SBMsg::Svc(SBSvcMsg::U(UPayload{body, ..})) => body,
            _ => panic!("SoupBinTcp message is not Svc and/or UPayload, instead it is: {:?}", self),
        }
    }
}
impl<CltPayload, SvcPayload> From<SBCltMsg<CltPayload>> for SBMsg<CltPayload, SvcPayload>
where
    CltPayload: ByteSerializeStack+ByteDeserializeSlice<CltPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
    SvcPayload: ByteSerializeStack+ByteDeserializeSlice<SvcPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
{
    fn from(value: SBCltMsg<CltPayload>) -> Self {
        SBMsg::Clt(value)
    }
}
impl<CltPayload, SvcPayload> From<SBSvcMsg<SvcPayload>> for SBMsg<CltPayload, SvcPayload>
where
    CltPayload: ByteSerializeStack+ByteDeserializeSlice<CltPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
    SvcPayload: ByteSerializeStack+ByteDeserializeSlice<SvcPayload>+ByteSerializedLenOf+PartialEq+Clone+fmt::Debug,
{
    fn from(value: SBSvcMsg<SvcPayload>) -> Self {
        SBMsg::Svc(value)
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::{prelude::*, unittest::setup::model::*};
    use byteserde::prelude::*;

    use links_core::unittest::setup;

    #[test]
    fn test_soup_bin_clt() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp = clt_msgs_default();

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out = vec![];
        while !des.is_empty() {
            let msg = SBCltMsg::<SamplePayload>::byte_deserialize(&mut des).unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
    #[test]
    fn test_soup_bin_svc() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp = svc_msgs_default();

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out = vec![];
        while !des.is_empty() {
            let msg = SBSvcMsg::<SamplePayload>::byte_deserialize(&mut des).unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }

    #[test]
    fn test_soup_max_frame_size() {
        setup::log::configure();
        let msg_inp_clt = clt_msgs_default::<Nil>()
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        let msg_inp_svc = svc_msgs_default::<Nil>()
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        for (len, msg) in msg_inp_clt.iter() {
            info!("len: {:>3}, msg: {:?} ", len, msg);
        }
        for (len, msg) in msg_inp_svc.iter() {
            info!("len: {:>3}, msg: {:?} ", len, msg);
        }
        let max_frame_size_no_payload = std::cmp::max(
            msg_inp_clt.iter().map(|(len, _)| *len).max().unwrap(),
            msg_inp_svc.iter().map(|(len, _)| *len).max().unwrap(),
        );
        info!("max_frame_size_no_payload: {}", max_frame_size_no_payload);
        assert_eq!(
            max_frame_size_no_payload,
            MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG
        )
    }
}
