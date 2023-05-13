use std::default;
use std::fmt::{Debug, Display};

use byteserde::prelude::*;
use byteserde::utils::strings::ascii::{CharAscii, ConstCharAscii};

const LOGIN_REJECTED_PACKET_LENGTH: u16 = 31;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq)]
#[byteserde(endian = "be")]
pub struct LoginRejected {
    #[byteserde(replace(LOGIN_REJECTED_PACKET_LENGTH))]
    packet_length: u16,
    packet_type: ConstCharAscii<b'J'>,
    reject_reason_code: CharAscii,
}
impl LoginRejected
{
    pub fn new_not_authorized() -> Self {
        LoginRejected {
            packet_length: LOGIN_REJECTED_PACKET_LENGTH,
            packet_type: Default::default(),
            reject_reason_code: CharAscii::new(b'A'),
        }
    }
    pub fn new_session_not_available() -> Self {
        LoginRejected {
            packet_length: LOGIN_REJECTED_PACKET_LENGTH,
            packet_type: Default::default(),
            reject_reason_code: CharAscii::new(b'S'),
        }
    }
}


impl Debug for LoginRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginRejected")
            .field("packet_length", &self.packet_length)
            .field("packet_type", &self.packet_type)
            .field("reject_reason_code", &self.reject_reason_code)
            .finish()
    }
}
impl Display for LoginRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = if self.reject_reason_code == CharAscii::new(b'A') {
            "Not Authorized. Invalid username or password in the LoginRequest"
        } else if self.reject_reason_code == CharAscii::new(b'S') {
            "Session Not Available. Te requested session in the LoginRequest was not valid or not available"
        } else {
            "Unknown"
        };
        write!(f, "Login Rejected reason \"{}\"", msg)
    }
}


#[cfg(test)]
mod test {
    use super::LoginRejected;
    use crate::{
        protocol::soupbintcp3::login_rejected::LOGIN_REJECTED_PACKET_LENGTH, unittest::setup,
    };
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_login_rejected() {
        setup::log::configure();
        let msg_inp = LoginRejected::new_not_authorized();
        info!("msg_inp: {}", msg_inp);
        let msg_inp = LoginRejected::new_session_not_available();
        info!("msg_inp: {}", msg_inp);

        let msg_inp = LoginRejected {
            packet_length: 0,
            ..LoginRejected::new_not_authorized()
        };
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: LoginRejected = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(
            msg_out,
            LoginRejected {
                packet_length: LOGIN_REJECTED_PACKET_LENGTH,
                ..msg_inp
            }
        );
    }
}
