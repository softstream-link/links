use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt::{Debug, Display};

use crate::model::types::{PacketTypeLoginAccepted, SequenceNumber, SessionId};

pub const LOGIN_ACCEPTED_PACKET_LENGTH: u16 = 31; // packet_type/1 + session/10 + sequence_number/20
pub const LOGIN_ACCEPTED_BYTE_LEN: usize = LOGIN_ACCEPTED_PACKET_LENGTH as usize + 2;
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct LoginAccepted {
    packet_length: u16,
    packet_type: PacketTypeLoginAccepted,
    session_id: SessionId,
    sequence_number: SequenceNumber,
}
impl LoginAccepted {
    pub fn new(session_id: SessionId, sequence_number: SequenceNumber) -> LoginAccepted {
        LoginAccepted {
            packet_length: LOGIN_ACCEPTED_PACKET_LENGTH,
            packet_type: Default::default(),
            session_id,
            sequence_number,
        }
    }
}
impl Default for LoginAccepted {
    fn default() -> Self {
        LoginAccepted {
            packet_length: LOGIN_ACCEPTED_PACKET_LENGTH,
            packet_type: Default::default(),
            session_id: b"session #1".into(),
            sequence_number: 1_u64.into(),
        }
    }
}

impl Display for LoginAccepted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Login Accepted, your session \"{}\", next sequence number \"{}\"", self.session_id, self.sequence_number)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_core::unittest::setup;
    use log::info;

    #[test]
    fn test_login_accepted() {
        setup::log::configure();
        let msg_inp = LoginAccepted::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(LOGIN_ACCEPTED_BYTE_LEN, ser.len());
        assert_eq!(LOGIN_ACCEPTED_BYTE_LEN, msg_inp.byte_len());

        let msg_out: LoginAccepted = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
