use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;
use std::fmt;

use super::unsequenced_data::UnsequencedData;

pub const MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG: usize = 54;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(peek(2, 1))]
pub enum SBCltMsg<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> {
    #[byteserde(eq(PacketTypeCltHeartbeat::as_slice()))]
    HBeat(CltHeartbeat),
    #[byteserde(eq(PacketTypeDebug::as_slice()))]
    Dbg(crate::model::debug::Debug),
    #[byteserde(eq(PacketTypeLoginRequest::as_slice()))]
    Login(LoginRequest),
    #[byteserde(eq(PacketTypeLogoutRequest::as_slice()))]
    Logout(LogoutRequest),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    SData(SequencedData::<T>),
    #[byteserde(eq(PacketTypeUnsequencedData::as_slice()))]
    UData(UnsequencedData::<T>),
}
#[rustfmt::skip]
impl<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> SBCltMsg<T> {
    pub fn login(username: UserName, password: Password, id: Option<SessionId>, num: Option<SequenceNumber>, tmout: Option<TimeoutMs>) -> Self { 
        Self::Login( LoginRequest::new(username, password, id.unwrap_or_default(), num.unwrap_or_default(),tmout.unwrap_or_default())) 
    }
    pub fn logout() -> Self { SBCltMsg::Logout(LogoutRequest::default()) }
    pub fn hbeat() -> Self { SBCltMsg::HBeat(CltHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { SBCltMsg::Dbg(Debug::new(text)) }
    pub fn sdata(payload: T) -> Self { SBCltMsg::SData(SequencedData::new(payload)) }
    pub fn udata(payload: T) -> Self { SBCltMsg::UData(UnsequencedData::new(payload)) }
}
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(peek(2, 1))]
pub enum SBSvcMsg<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug>{
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
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    SData(SequencedData::<T>),
    #[byteserde(eq(PacketTypeUnsequencedData::as_slice()))]
    UData(UnsequencedData::<T>),
}
#[rustfmt::skip]
impl<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> SBSvcMsg<T> {
    pub fn end() -> Self { Self::End(EndOfSession::default()) }
    pub fn login_acc(id: SessionId, num: SequenceNumber) -> Self { Self::LoginAcc(LoginAccepted::new(id, num)) }
    pub fn login_rej_not_auth() -> Self { Self::LoginRej(LoginRejected::not_authorized()) }
    pub fn login_rej_ses_not_avail() -> Self { Self::LoginRej(LoginRejected::session_not_available()) }
    pub fn hbeat() -> Self { Self::HBeat(SvcHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { Self::Dbg(Debug::new(text)) }
    pub fn sdata(payload: T) -> Self { Self::SData(SequencedData::new(payload)) }
    pub fn udata(payload: T) -> Self { Self::UData(UnsequencedData::new(payload)) }
}

#[rustfmt::skip]
#[derive(Debug, Clone, PartialEq)]
pub enum SBMsg<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug>{
    Clt(SBCltMsg<T>),
    Svc(SBSvcMsg<T>),
}
#[rustfmt::skip]
impl<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> From<SBCltMsg<T>> for SBMsg<T>{
    fn from(value: SBCltMsg<T>) -> Self {
        SBMsg::Clt(value)
    }
}
#[rustfmt::skip]
impl<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> From<SBSvcMsg<T>> for SBMsg<T>{
    fn from(value: SBSvcMsg<T>) -> Self {
        SBMsg::Svc(value)
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::{prelude::*, unittest::setup::model::*};
    use byteserde::prelude::*;

    use links_testing::unittest::setup::log::configure;

    use crate::unittest::setup;
    #[test]
    fn test_soup_bin_clt() {
        configure();
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
        configure();
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
        configure();
        let msg_inp_clt = setup::model::clt_msgs_default::<NoPayload>()
            .into_iter()
            .map(|msg| (msg.byte_len(), msg))
            .collect::<Vec<_>>();
        let msg_inp_svc = setup::model::svc_msgs_default::<NoPayload>()
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
        assert_eq!(max_frame_size_no_payload, MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG)
    }
}
