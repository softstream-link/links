use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt::Display;

use crate::model::types::PacketTypeEndOfSession;

pub const END_OF_SESSION_PACKET_LENGTH: u16 = 1;
pub const END_OF_SESSION_BYTE_LEN: usize = END_OF_SESSION_PACKET_LENGTH as usize + 2;
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct EndOfSession {
    packet_length: u16,
    packet_type: PacketTypeEndOfSession,
}
impl Default for EndOfSession {
    fn default() -> Self {
        EndOfSession {
            packet_length: END_OF_SESSION_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}
impl Display for EndOfSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "End of Session")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_core::unittest::setup;
    use log::info;

    #[test]
    fn test_end_of_session() {
        setup::log::configure();

        let msg_inp = EndOfSession::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(END_OF_SESSION_BYTE_LEN, ser.len());
        assert_eq!(END_OF_SESSION_BYTE_LEN, msg_inp.byte_len());

        let msg_out: EndOfSession = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
