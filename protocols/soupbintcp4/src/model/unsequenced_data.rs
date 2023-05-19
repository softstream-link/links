use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeUnsequenceData;

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
pub struct UnsequencedDataHeader {
    packet_length: u16,
    packet_type: PacketTypeUnsequenceData,
}

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
pub struct UnsequencedData {
    header: UnsequencedDataHeader,
    body: Vec<u8>,
}

impl UnsequencedData {
    pub fn new(body: &[u8]) -> Self {
        UnsequencedData {
            header: UnsequencedDataHeader {
                packet_length: (body.len() + PacketTypeUnsequenceData::size()) as u16,
                packet_type: PacketTypeUnsequenceData::default(),
            },
            body: body.into(),
        }
    }
}

impl Display for UnsequencedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unsequenced Data 0x{:02x?}", self.body)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_unsequenced_data() {
        setup::log::configure();

        let msg_inp = UnsequencedData::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: UnsequencedData = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}