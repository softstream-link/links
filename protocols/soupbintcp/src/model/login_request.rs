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
    pub username: UserName,
    pub password: Password,
    pub session_id: SessionId,
    pub sequence_number: SequenceNumber,
    pub hbeat_timeout: TimeoutMs,
}
impl LoginRequest {
    pub fn new(
        username: UserName,
        password: Password,
        session_id: SessionId,
        sequence_number: SequenceNumber,
        hbeat_timeout: TimeoutMs,
    ) -> LoginRequest {
        LoginRequest {
            packet_length: LOGIN_REQUEST_PACKET_LENGTH,
            packet_type: Default::default(),
            username,
            password,
            session_id,
            sequence_number,
            hbeat_timeout,
        }
    }
}

// obfuscate password
impl Debug for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut obfs = self.clone();
        obfs.password = b"********".as_slice().into();
        f.debug_struct("LoginRequest")
            .field("packet_length", &obfs.packet_length)
            .field("packet_type", &obfs.packet_type)
            .field("username", &obfs.username)
            .field("password", &obfs.password)
            .field("session_id", &obfs.session_id)
            .field("sequence_number", &obfs.sequence_number)
            .field("hbeat_timeout", &obfs.hbeat_timeout)
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
            self.username,
            self.session_id,
            self.sequence_number,
            self.hbeat_timeout,
        
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
