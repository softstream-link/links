use std::fmt::{Debug, Display};

use byteserde::prelude::*;
use byteserde::utils::strings::ascii::{ConstCharAscii, StringAsciiFixed};

const LOGING_ACCEPTED_PACKET_LENGTH: u16 = 31;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq)]
#[byteserde(endian = "be")]
pub struct LoginAccepted {
    #[byteserde(replace(LOGING_ACCEPTED_PACKET_LENGTH))]
    packet_length: u16,
    packet_type: ConstCharAscii<b'A'>,
    session: StringAsciiFixed<10, b' ', true>,
    sequence_number: StringAsciiFixed<20, b' ', true>,
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
impl Debug for LoginAccepted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginAccepted")
            .field("packet_length", &self.packet_length)
            .field("packet_type", &self.packet_type)
            .field("session", &self.session)
            .field("sequence_number", &self.sequence_number)
            .finish()
    }
}
impl Display for LoginAccepted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Login Accepted, your session \"{}\", next sequence number \"{}\"", self.session, self.sequence_number)
    }
}

#[cfg(test)]
mod test {
    use super::LoginAccepted;
    use crate::{
        protocol::soupbintcp3::login_accepted::LOGING_ACCEPTED_PACKET_LENGTH, unittest::setup,
    };
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_login_accepted() {
        setup::log::configure();
        let msg_inp = LoginAccepted {
            packet_length: 0,
            ..Default::default()
        };
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: LoginAccepted = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(
            msg_out,
            LoginAccepted {
                packet_length: LOGING_ACCEPTED_PACKET_LENGTH,
                ..msg_inp
            }
        );
    }
}
