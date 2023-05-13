use std::fmt::{Debug, Display};

use byteserde::{
    prelude::*,
    utils::strings::ascii::{ConstCharAscii, StringAsciiFixed},
};

const LOGIN_REQUEST_PACKET_LENGTH: u16 = 52;
const S: u8 = b' ';
const R: bool = true;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq)]
#[byteserde(endian = "be")]
pub struct LoginRequest {
    packet_length: u16,
    packet_type: ConstCharAscii<b'L'>,
    username: StringAsciiFixed<6, S, R>,
    password: StringAsciiFixed<10, S, R>,
    requested_session: StringAsciiFixed<10, S, R>,
    requested_sequence_number: StringAsciiFixed<20, S, R>,
    heartbeat_timeout_ms: StringAsciiFixed<5, S, R>,
}
impl LoginRequest {
    fn new(
        username: StringAsciiFixed<6, b' ', true>,
        password: StringAsciiFixed<10, S, R>,
        requested_session: StringAsciiFixed<10, S, R>,
        requested_sequence_number: StringAsciiFixed<20, S, R>,
        heartbeat_timeout_ms: StringAsciiFixed<5, S, R>,
    ) -> LoginRequest {
        LoginRequest {
            packet_length: LOGIN_REQUEST_PACKET_LENGTH,
            packet_type: Default::default(),
            username,
            password,
            requested_session,
            requested_sequence_number,
            heartbeat_timeout_ms,
        }
    }
}

// obfiscate password
impl Debug for LoginRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pass: StringAsciiFixed<5, S, R> = b"********".as_slice().into();
        f.debug_struct("LoginRequest")
            .field("packet_length", &self.packet_length)
            .field("packet_type", &self.packet_type)
            .field("username", &self.username)
            .field("password", &pass)
            .field("requested_session", &self.requested_session)
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
            self.username,
            self.requested_session,
            self.requested_sequence_number,
            self.heartbeat_timeout_ms,
        
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
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
        assert_eq!(ser.len() - 2, LOGIN_REQUEST_PACKET_LENGTH as usize);

        let msg_out: LoginRequest = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
