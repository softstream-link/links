use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::prelude::*;
use std::fmt;

use super::unsequenced_data::UnsequencedData;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(peek(2, 1))]
pub enum SoupBin<T>
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
impl<PAYLOAD> SoupBin<PAYLOAD>
where PAYLOAD: ByteSerializeStack + ByteDeserializeSlice<PAYLOAD> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug
{
    pub fn clt_hbeat() -> Self { SoupBin::CltHBeat(CltHeartbeat::default()) }
    pub fn svc_hbeat() -> Self { SoupBin::SvcHBeat(SvcHeartbeat::default()) }
    pub fn dbg(text: &[u8]) -> Self { SoupBin::Dbg(Debug::new(text)) }
    pub fn end() -> Self { SoupBin::End(EndOfSession::default()) }
    pub fn login_acc(id: SessionId, num: SequenceNumber) -> Self { 
        SoupBin::LoginAcc(LoginAccepted::new(id, num)) 
    }
    
    pub fn login_rej_not_auth() -> Self { SoupBin::LoginRej(LoginRejected::not_authorized()) }
    pub fn login_rej_ses_not_avail() -> Self { SoupBin::LoginRej(LoginRejected::session_not_available()) }

    pub fn login_req(usr: UserName, pwd: Password, id: Option<SessionId>, num: Option<SequenceNumber>, tmout: Option<TimeoutMs>) -> Self { 
        SoupBin::LoginReq(
            LoginRequest::new(
                usr, 
                pwd,
                id.unwrap_or_default(), 
                num.unwrap_or_default(),
                tmout.unwrap_or_default()
            )
        ) 
    }
    pub fn logout_req() -> Self { SoupBin::LogoutReq(LogoutRequest::default()) }
    pub fn sdata<T>(payload: T) -> SoupBin<T> 
    where T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug{ 
        SoupBin::SData(SequencedData::new(payload)) 
    }
    pub fn udata<T>(payload: T) -> SoupBin<T> 
    where T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug{ 
        SoupBin::UData(UnsequencedData::new(payload)) 
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
            SoupBin::CltHBeat(CltHeartbeat::default()),
            SoupBin::SvcHBeat(SvcHeartbeat::default()),
            SoupBin::Dbg(Debug::default()),
            SoupBin::End(EndOfSession::default()),
            SoupBin::LoginReq(LoginRequest::default()),
            SoupBin::LoginAcc(LoginAccepted::default()),
            SoupBin::LoginRej(LoginRejected::not_authorized()),
            SoupBin::LogoutReq(LogoutRequest::default()),
            SoupBin::SData(SequencedData::<SamplePayload>::default()),
            SoupBin::UData(UnsequencedData::<SamplePayload>::default()),
        ];

        for msg in msg_inp.iter() {
            info!("msg_inp: {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut des = ByteDeserializerSlice::new(ser.as_slice());
        let mut msg_out: Vec<SoupBin<SamplePayload>> = vec![];
        while !des.is_empty() {
            let msg = SoupBin::<SamplePayload>::byte_deserialize(&mut des).unwrap();
            info!("msg_out: {:?}", msg);
            msg_out.push(msg);
        }
        assert_eq!(msg_inp, msg_out);
    }
}
