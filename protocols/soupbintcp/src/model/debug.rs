use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use byteserde_types::prelude::*;
use std::fmt;

use super::types::PacketTypeDebug;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(endian = "be")]
pub struct Debug {
    #[byteserde(replace( packet_type.byte_len() + text.byte_len() ))]
    packet_length: u16,
    packet_type: PacketTypeDebug,
    #[byteserde(deplete ( packet_length as usize - packet_type.byte_len() ))]
    text: StringAscii,
}

impl Debug {
    pub fn new(text: &[u8]) -> Self {
        Debug {
            packet_length: (text.len() + PacketTypeDebug::byte_size()) as u16,
            text: text.into(),
            ..Default::default()
        }
    }
}
impl Default for Debug {
    fn default() -> Self {
        let text = b"This is a default debug message text";
        Debug {
            packet_length: (text.len() + PacketTypeDebug::byte_size()) as u16,
            text: text.into(),
            packet_type: Default::default(),
        }
    }
}
impl fmt::Display for Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_testing::unittest::setup;
    use log::info;

    #[test]
    fn test_debug() {
        setup::log::configure();

        let msg_inp = Debug::default();
        let expected_packet_len: u16 = (msg_inp.text.len() + msg_inp.packet_type.byte_len()) as u16;
        let expected_byte_len: usize = expected_packet_len as usize + 2;

        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(expected_byte_len, ser.len());
        assert_eq!(expected_byte_len, msg_inp.byte_len());

        let msg_out: Debug = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(
            msg_out,
            Debug {
                packet_length: expected_packet_len,
                ..msg_inp
            }
        );
    }
}
