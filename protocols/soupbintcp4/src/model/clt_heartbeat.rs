use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeClientHeartbeat;

const CLIENT_HEARTBEAT_PACKET_LENGTH: u16 = 3;

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct ClientHeartbeat {
    packet_length: u16,
    packet_type: PacketTypeClientHeartbeat,
}

impl Default for ClientHeartbeat {
    fn default() -> Self {
        ClientHeartbeat {
            packet_length: CLIENT_HEARTBEAT_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}
impl Display for ClientHeartbeat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client Heartbeat")
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

        let msg_inp = ClientHeartbeat::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: ClientHeartbeat = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
