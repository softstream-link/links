use std::fmt::{Debug, Display};

use byteserde::prelude::*;
use byteserde::utils::strings::ascii::ConstCharAscii;

const LOGOUT_REQUEST_PACKET_LENGTH: u16 = 1;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct LogoutRequest {
    packet_length: u16,
    packet_type: ConstCharAscii<b'O'>,
}
impl Default for LogoutRequest {
    fn default() -> Self {
        LogoutRequest {
            packet_length: LOGOUT_REQUEST_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}

impl Display for LogoutRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Logout Request")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;

    #[test]
    fn test_login_accepted() {
        setup::log::configure();
        let msg_inp = Default::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(ser.len() - 2, LOGOUT_REQUEST_PACKET_LENGTH as usize);

        let msg_out: LogoutRequest = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
