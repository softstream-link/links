use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt::{Debug, Display};

use super::types::PacketTypeLogoutRequest;

pub const LOGOUT_REQUEST_PACKET_LENGTH: u16 = 1;
pub const LOGOUT_REQUEST_BYTE_LEN: usize = LOGOUT_REQUEST_PACKET_LENGTH as usize + 2;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct LogoutRequest {
    packet_length: u16,
    packet_type: PacketTypeLogoutRequest,
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
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_login_accepted() {
        setup::log::configure();
        let msg_inp = LogoutRequest::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(LOGOUT_REQUEST_BYTE_LEN, ser.len());
        assert_eq!(LOGOUT_REQUEST_BYTE_LEN, msg_inp.byte_len());

        let msg_out: LogoutRequest = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
