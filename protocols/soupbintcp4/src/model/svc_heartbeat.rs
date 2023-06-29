use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeSvcHeartbeat;

const SERVER_HEARTBEAT_PACKET_LENGTH: u16 = 1;

#[derive(ByteSerializeStack, ByteDeserializeSlice, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SvcHeartbeat {
    packet_length: u16,
    packet_type: PacketTypeSvcHeartbeat,
}

impl Default for SvcHeartbeat {
    fn default() -> Self {
        SvcHeartbeat {
            packet_length: SERVER_HEARTBEAT_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}
impl Display for SvcHeartbeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server Heartbeat")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_server_heartbeat() {
        setup::log::configure();

        let msg_inp = SvcHeartbeat::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: SvcHeartbeat = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
