use std::fmt::{Debug, Display};

use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use crate::model::types::{PacketTypeLoginRejected, RejectReason};

pub const LOGIN_REJECTED_PACKET_LENGTH: u16 = 2;
pub const LOGIN_REJECTED_BYTE_LEN: usize = LOGIN_REJECTED_PACKET_LENGTH as usize + 2;
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct LoginRejected {
    packet_length: u16,
    packet_type: PacketTypeLoginRejected,
    reject_reason_code: RejectReason,
}
impl LoginRejected {
    pub fn not_authorized() -> Self {
        LoginRejected {
            packet_length: LOGIN_REJECTED_PACKET_LENGTH,
            packet_type: Default::default(),
            reject_reason_code: RejectReason::new(b'A'),
        }
    }
    pub fn session_not_available() -> Self {
        LoginRejected {
            packet_length: LOGIN_REJECTED_PACKET_LENGTH,
            packet_type: Default::default(),
            reject_reason_code: RejectReason::new(b'S'),
        }
    }
    pub fn is_not_authorized(&self) -> bool {
        self.reject_reason_code == LoginRejected::not_authorized().reject_reason_code
    }
    pub fn is_session_not_available(&self) -> bool {
        self.reject_reason_code == LoginRejected::session_not_available().reject_reason_code
    }
}

impl Display for LoginRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = if self.reject_reason_code == RejectReason::new(b'A') {
            "Not Authorized. Invalid username or password in the LoginRequest"
        } else if self.reject_reason_code == RejectReason::new(b'S') {
            "Session Not Available. Te requested session in the LoginRequest was not valid or not available"
        } else {
            "Unknown"
        };
        write!(f, "Login Rejected reason \"{}\"", msg)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_core::unittest::setup;
    use log::info;

    #[test]
    fn test_login_rejected() {
        setup::log::configure();

        let msg_inp = LoginRejected::not_authorized();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(LOGIN_REJECTED_BYTE_LEN, ser.len());
        assert_eq!(LOGIN_REJECTED_BYTE_LEN, msg_inp.byte_len());

        let msg_inp = LoginRejected::session_not_available();
        info!("msg_inp: {}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(ser.len() - 2, LOGIN_REJECTED_PACKET_LENGTH as usize);

        let msg_out: LoginRejected = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
