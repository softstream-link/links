use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeServerHeartbeat;

const SERVER_HEARTBEAT_PACKET_LENGTH: u16 = 3;

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct ServerHeartbeat {
    packet_length: u16,
    packet_type: PacketTypeServerHeartbeat,
}

impl Default for ServerHeartbeat {
    fn default() -> Self {
        ServerHeartbeat {
            packet_length: SERVER_HEARTBEAT_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}
impl Display for ServerHeartbeat {
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

        let msg_inp = ServerHeartbeat::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: ServerHeartbeat = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
