use std::fmt::Display;
use byteserde_derive::{ByteSerializeStack, ByteDeserialize};

use super::types::PacketTypeEndOfSession;

const END_OF_SESSION_PACKET_LENGTH: u16 = 1;
#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
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
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_end_of_session() {
        setup::log::configure();

        let msg_inp = EndOfSession::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: EndOfSession = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
