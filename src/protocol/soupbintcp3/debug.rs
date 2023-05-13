use core;
use std::fmt::Display;

use byteserde::prelude::*;
use byteserde::utils::strings::ascii::{ConstCharAscii, StringAscii};

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, core::fmt::Debug)]
#[byteserde(endian = "be")]
pub struct Debug {
    #[byteserde(replace( (packet_type.len() + text.len()) as u16 ))]
    packet_length: u16,
    packet_type: ConstCharAscii<b'+'>,
    #[byteserde(length ( packet_length as usize - packet_type.len() ))]
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
impl Display for Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;

    #[test]
    fn test_debug() {
        setup::log::configure();

        let expected_len: u16 = 37;

        let msg_inp = Debug::default();
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(ser.len() - 2, expected_len as usize);

        let msg_out: Debug = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(
            msg_out,
            Debug {
                packet_length: expected_len,
                ..msg_inp
            }
        );
    }
}
