use byteserde::prelude::*;
use byteserde_derive::{ByteSerializeStack, ByteDeserializeSlice};

use crate::prelude::*;

#[derive(ByteSerializeStack, ByteDeserializeSlice,  Debug, PartialEq)]
// #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, Debug, PartialEq)]
#[byteserde(peek(2, 1))]
pub enum SoupBin<T> 
where T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq{
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
    UData(UnsequencedDataVec),
}

// impl ByteDeserializeSlice<SoupBin> for SoupBin {
//     fn byte_deserialize(des: &mut ByteDeserializer) -> byteserde::prelude::Result<SoupBin> {
//         let peek = |start, len| -> Result<&[u8]> {
//             let p = des.peek_bytes_slice(len + start)?;
//             Ok(&p[start..])
//         };
//         let msg_type = peek(2, 1)?;
//         if msg_type == PacketTypeCltHeartbeat::as_slice() {
//             return Ok(Self::CltHBeat(CltHeartbeat::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeSvcHeartbeat::as_slice() {
//             return Ok(SoupBin::SvcHBeat(SvcHeartbeat::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeDebug::as_slice() {
//             return Ok(Self::Dbg(crate::prelude::Debug::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeEndOfSession::as_slice() {
//             return Ok(Self::End(EndOfSession::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeLoginAccepted::as_slice() {
//             return Ok(Self::LoginAcc(LoginAccepted::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeLoginRejected::as_slice() {
//             return Ok(Self::LoginRej(LoginRejected::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeLoginRequest::as_slice() {
//             return Ok(Self::LoginReq(LoginRequest::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeLogoutRequest::as_slice() {
//             return Ok(Self::LogoutReq(LogoutRequest::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeSequenceData::as_slice(){
//             return Ok(Self::SData(SequencedData::byte_deserialize(des)?));
//         }
//         if msg_type == PacketTypeUnsequenceData::as_slice(){
//             return Ok(Self::UData(UnsequencedData::byte_deserialize(des)?));
//         }

//         Err(SerDesError {
//             message: "blah".to_owned(), // TODO finish error message
//         })
//     }
// }
