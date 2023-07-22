use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;
use std::fmt;

use super::unsequenced_data::UnsequencedData;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(peek(2, 1))]
pub enum SBMsg<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug, // TODO can this be done via super trait?
{
    #[byteserde(eq(PacketTypeCltHeartbeat::as_slice()))]
    CltHBeat(CltHeartbeat),
    #[byteserde(eq(PacketTypeSvcHeartbeat::as_slice()))]
    SvcHBeat(SvcHeartbeat),
    #[byteserde(eq(PacketTypeDebug::as_slice()))]
    Dbg(crate::model::debug::Debug),
    #[byteserde(eq(PacketTypeEndOfSession::as_slice()))]
    End(EndOfSession),
    #[byteserde(eq(PacketTypeLoginAccepted::as_slice()))]
    LoginAcc(LoginAccepted),
    #[byteserde(eq(PacketTypeLoginRejected::as_slice()))]
    LoginRej(LoginRejected),
    #[byteserde(eq(PacketTypeLoginRequest::as_slice()))]
    LoginReq(LoginRequest),
    #[byteserde(eq(PacketTypeLogoutRequest::as_slice()))]
    LogoutReq(LogoutRequest),
    #[byteserde(eq(PacketTypeSequenceData::as_slice()))]
    SData(SequencedData::<T>),
    #[byteserde(eq(PacketTypeUnsequenceData::as_slice()))]
    UData(UnsequencedData::<T>),
}
#[rustfmt::skip]
impl<PAYLOAD> SBMsg<PAYLOAD>
where PAYLOAD: ByteSerializeStack + ByteDeserializeSlice<PAYLOAD> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug
{
    pub fn clt_hbeat() -> Self { SBMsg::CltHBeat(CltHeartbeat::default()) }
    pub fn svc_hbeat() -> Self { SBMsg::SvcHBeat(SvcHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { SBMsg::Dbg(Debug::new(text)) }
    pub fn end() -> Self { SBMsg::End(EndOfSession::default()) }
    pub fn login_acc(id: SessionId, num: SequenceNumber) -> Self { 
        SBMsg::LoginAcc(LoginAccepted::new(id, num)) 
    }
    
    pub fn login_rej_not_auth() -> Self { SBMsg::LoginRej(LoginRejected::not_authorized()) }
    pub fn login_rej_ses_not_avail() -> Self { SBMsg::LoginRej(LoginRejected::session_not_available()) }

    pub fn login_req(usr: UserName, pwd: Password, id: Option<SessionId>, num: Option<SequenceNumber>, tmout: Option<TimeoutMs>) -> Self { 
        SBMsg::LoginReq(
            LoginRequest::new(
                usr, 
                pwd,
                id.unwrap_or_default(), 
                num.unwrap_or_default(),
                tmout.unwrap_or_default()
            )
        ) 
    }
    pub fn logout_req() -> Self { SBMsg::LogoutReq(LogoutRequest::default()) }
    pub fn sdata<T>(payload: T) -> SBMsg<T> 
    where T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug{ 
        SBMsg::SData(SequencedData::new(payload)) 
    }
    pub fn udata<T>(payload: T) -> SBMsg<T> 
    where T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug{ 
        SBMsg::UData(UnsequencedData::new(payload)) 
    }

}

#[cfg(test)]
mod test {

    use log::info;

    use super::*;

    use crate::{model::payload::SamplePayload, unittest::setup};

    #[test]
    fn test_soup_bin() {
        setup::log::configure();
        let mut ser = ByteSerializerStack::<1024>::default();
        let msg_inp = vec![
            SBMsg::CltHBeat(CltHeartbeat::default()),
            SBMsg::SvcHBeat(SvcHeartbeat::default()),
            SBMsg::Dbg(Debug::default()),
            SBMsg::End(EndOfSession::default()),
            SBMsg::LoginReq(LoginRequest::default()),
            SBMsg::LoginAcc(LoginAccepted::default()),
            SBMsg::LoginRej(LoginRejected::not_authorized()),
            SBMsg::LogoutReq(LogoutRequest::default()),
            SBMsg::SData(SequencedData::<SamplePayload>::default()),
            SBMsg::UData(UnsequencedData::<SamplePayload>::default()),
        ];

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out: Vec<SBMsg<SamplePayload>> = vec![];
        while !des.is_empty() {
            let msg = SBMsg::<SamplePayload>::byte_deserialize(&mut des).unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
