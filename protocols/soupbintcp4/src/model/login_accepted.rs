use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::{Debug, Display};

use super::types::{PacketTypeLoginAccepted, SequenceNumber, SessionId};

const LOGING_ACCEPTED_PACKET_LENGTH: u16 = 31;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct LoginAccepted {
    packet_length: u16,
    packet_type: PacketTypeLoginAccepted,
    session: SessionId,
    sequence_number: SequenceNumber,
}
impl Default for LoginAccepted {
    fn default() -> Self {
        LoginAccepted {
            packet_length: LOGING_ACCEPTED_PACKET_LENGTH,
            packet_type: Default::default(),
            session: b"session #1".into(),
            sequence_number: 1_u64.into(),
        }
    }
}

impl Display for LoginAccepted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Login Accepted, your session \"{}\", next sequence number \"{}\"",
            self.session, self.sequence_number
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_login_accepted() {
        setup::log::configure();
        let msg_inp = Default::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(ser.len() - 2, LOGING_ACCEPTED_PACKET_LENGTH as usize);

        let msg_out: LoginAccepted = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
