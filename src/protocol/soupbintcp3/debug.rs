use core;
use std::fmt::Display;

use byteserde::prelude::*;
use byteserde::utils::strings::ascii::{ConstCharAscii, StringAscii};

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq)]
#[byteserde(endian = "be")]
pub struct Debug {
    #[byteserde(replace( (packet_type.len() + text.len()) as u16 ))]
    packet_length: u16,
    packet_type: ConstCharAscii<b'+'>,
    // #[byteserde(length ( (packet_length as usize - packet_type.len()) as usize ))] // TODO add necessary vars in macro to make this possible
    #[byteserde(length ( (packet_length - 1 ) as usize ))]
    text: StringAscii,
}
impl Debug {
    pub fn new(msg: &[u8]) -> Self {
        Debug {
            packet_length: Default::default(),
            packet_type: Default::default(),
            text: msg.into(),
        }
    }
}
impl Default for Debug {
    fn default() -> Self {
        Debug {
            packet_length: 0,
            packet_type: Default::default(),
            text: b"This is a default debug message text".into(),
        }
    }
}
impl core::fmt::Debug for Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Debug")
            .field("packet_length", &self.packet_length)
            .field("packet_type", &self.packet_type.to_char())
            .field("text", &self.text.to_string())
            .finish()
    }
}
impl Display for Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod test {
    use super::Debug;
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_debug() {
        setup::log::configure();
        let msg_inp = Debug::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: Debug = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(
            msg_out,
            Debug {
                packet_length: 37,
                ..msg_inp
            }
        );
    }
}
