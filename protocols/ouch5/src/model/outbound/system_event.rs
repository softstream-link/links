use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct SystemEvent {
    packet_type: PacketTypeSystemEvent,
    timestamp: Timestamp,
    event_code: EventCode,
}

impl Default for SystemEvent {
    fn default() -> Self {
        Self {
            packet_type: PacketTypeSystemEvent::default(),
            timestamp: Timestamp::default(),
            event_code: EventCode::start_of_day(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::prelude::*;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let msg_inp = SystemEvent::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: SystemEvent = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
