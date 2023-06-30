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
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug,
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

#[cfg(test)]
mod test {

    use log::info;

    use super::*;

    use crate::{unittest::setup, model::sample_payload::SamplePayload};

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
