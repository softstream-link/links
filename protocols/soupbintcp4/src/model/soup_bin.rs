use byteserde::prelude::*;

use crate::prelude::*;

#[derive(Debug, PartialEq)]
pub enum SoupBin {
    CltHeartBeat(CltHeartbeat),
    SvcHeartbeat(SvcHeartbeat),
    Debug(crate::model::debug::Debug),
    EndOfSession(EndOfSession),
    LoginAccepted(LoginAccepted),
    LoginRejected(LoginRejected),
    LoginRequest(LoginRequest),
    LogoutRequest(LogoutRequest),
    SequencedData(SequencedData),
    UnsequencedData(UnsequencedData),
}
impl ByteSerializeStack for SoupBin {
    fn byte_serialize_stack<const CAP: usize>(
        &self,
        ser: &mut byteserde::ser::ByteSerializerStack<CAP>,
    ) -> byteserde::prelude::Result<()> {
        Ok(match self {
            Self::CltHeartBeat(msg) => msg.byte_serialize_stack(ser)?,
            Self::SvcHeartbeat(msg) => msg.byte_serialize_stack(ser)?,
            Self::Debug(msg) => msg.byte_serialize_stack(ser)?,
            Self::EndOfSession(msg) => msg.byte_serialize_stack(ser)?,
            Self::LoginAccepted(msg) => msg.byte_serialize_stack(ser)?,
            Self::LoginRejected(msg) => msg.byte_serialize_stack(ser)?,
            Self::LoginRequest(msg) => msg.byte_serialize_stack(ser)?,
            Self::LogoutRequest(msg) => msg.byte_serialize_stack(ser)?,
            Self::SequencedData(msg) => msg.byte_serialize_stack(ser)?,
            Self::UnsequencedData(msg) => msg.byte_serialize_stack(ser)?,
        })
    }
}

impl ByteDeserialize<SoupBin> for SoupBin {
    fn byte_deserialize(des: &mut ByteDeserializer) -> byteserde::prelude::Result<SoupBin> {
        let peek = |start, len| -> Result<&[u8]> {
            let p = des.peek_bytes_slice(len + start)?;
            Ok(&p[start..])
        };
        let msg_type = peek(2, 1)?;
        if msg_type == PacketTypeClientHeartbeat::as_slice() {
            return Ok(SoupBin::CltHeartBeat(CltHeartbeat::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeServerHeartbeat::as_slice() {
            return Ok(SoupBin::SvcHeartbeat(SvcHeartbeat::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeDebug::as_slice() {
            return Ok(Self::Debug(crate::prelude::Debug::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeEndOfSession::as_slice() {
            return Ok(Self::EndOfSession(EndOfSession::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeLoginAccepted::as_slice() {
            return Ok(Self::LoginAccepted(LoginAccepted::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeLoginRejected::as_slice() {
            return Ok(Self::LoginRejected(LoginRejected::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeLoginRequest::as_slice() {
            return Ok(Self::LoginRequest(LoginRequest::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeLogoutRequest::as_slice() {
            return Ok(Self::LogoutRequest(LogoutRequest::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeSequenceData::as_slice(){
            return Ok(Self::SequencedData(SequencedData::byte_deserialize(des)?));
        }
        if msg_type == PacketTypeUnsequenceData::as_slice(){
            return Ok(Self::UnsequencedData(UnsequencedData::byte_deserialize(des)?));
        }

        Err(SerDesError {
            message: "blah".to_owned(), // TODO finish error message
        })
    }
}
