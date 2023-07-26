use std::fmt::{Debug, Display};
use byteserde_derive::{ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf};

use super::types::{PacketTypeLoginRequest, UserName, Password, SessionId, SequenceNumber, TimeoutMs};

// packet_type/1 + usr/6 + pwd/10 + requested_session/10 + requested_sequence_number/20 + heartbeat_timeout_ms/5
pub const LOGIN_REQUEST_PACKET_LENGTH: u16 = 52; 
pub const LOGIN_REQUEST_BYTE_LEN: usize = LOGIN_REQUEST_PACKET_LENGTH as usize + 2;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf,PartialEq, Clone)]
#[byteserde(endian = "be")]
pub struct LoginRequest {
    packet_length: u16,
    packet_type: PacketTypeLoginRequest,
    usr: UserName,
    pwd: Password,
    requested_session_id: SessionId,
    requested_sequence_number: SequenceNumber,
    heartbeat_timeout_ms: TimeoutMs,
}
impl LoginRequest {
    pub fn new(
        username: UserName,
        password: Password,
        requested_session_id: SessionId,
        requested_sequence_number: SequenceNumber,
        heartbeat_timeout_ms: TimeoutMs,
    ) -> LoginRequest {
        LoginRequest {
            packet_length: LOGIN_REQUEST_PACKET_LENGTH,
            packet_type: Default::default(),
            usr: username,
            pwd: password,
            requested_session_id,
            requested_sequence_number,
            heartbeat_timeout_ms,
        }
    }
}

// obfiscate password
impl Debug for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pass: Password = b"********".as_slice().into();
        f.debug_struct("LoginRequest")
            .field("packet_length", &self.packet_length)
            .field("packet_type", &self.packet_type)
            .field("usr", &self.usr)
            .field("pwd", &pass)
            .field("requested_session", &self.requested_session_id)
            .field("requested_sequence_number", &self.requested_sequence_number)
            .field("heartbeat_timeout_ms", &self.heartbeat_timeout_ms)
            .finish()
    }
}
impl Default for LoginRequest {
    fn default() -> Self {
        LoginRequest::new(
            b"dummy".as_slice().into(),
            b"dummy".as_slice().into(),
            b"session #1".into(),
            1_u64.into(),
            5000_u16.into(),
        )
    }
}

impl Display for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Login Request, as username \"{}\" requested for session \"{}\", sequence \"{}\", heartbeat timeout {}ms",
            self.usr,
            self.requested_session_id,
            self.requested_sequence_number,
            self.heartbeat_timeout_ms,
        
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_testing::unittest::setup;
    use log::info;

    use super::LoginRequest;
    #[test]
    fn test_login_request() {
        setup::log::configure();
        let msg_inp = LoginRequest::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);

        let msg_inp = LoginRequest::new(
            b"abcdef".into(),
            b"1234567890".into(), 
b"session #1".into(),
1_u64.into(),
5000_u16.into());
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(LOGIN_REQUEST_BYTE_LEN, ser.len());
        assert_eq!(LOGIN_REQUEST_BYTE_LEN, msg_inp.byte_len());

        let msg_out: LoginRequest = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
