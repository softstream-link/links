use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeCltHeartbeat;

const CLIENT_HEARTBEAT_PACKET_LENGTH: u16 = 1;

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct CltHeartbeat {
    packet_length: u16,
    packet_type: PacketTypeCltHeartbeat,
}

impl Default for CltHeartbeat {
    fn default() -> Self {
        CltHeartbeat {
            packet_length: CLIENT_HEARTBEAT_PACKET_LENGTH,
            packet_type: Default::default(),
        }
    }
}
impl Display for CltHeartbeat {
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

        let msg_inp = CltHeartbeat::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: CltHeartbeat = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
