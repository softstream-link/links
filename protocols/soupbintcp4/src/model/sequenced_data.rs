use bytes::Bytes;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeBytes, ByteDeserializeSlice, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeSequenceData;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteDeserializeBytes, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedDataHeader {
    packet_length: u16,
    packet_type: PacketTypeSequenceData,
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedData {
    header: SequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: Vec<u8>,
}

impl SequencedData {
    pub fn new(body: &[u8]) -> Self {
        SequencedData {
            header: SequencedDataHeader {
                packet_length: (body.len() + PacketTypeSequenceData::byte_size()) as u16,
                packet_type: PacketTypeSequenceData::default(),
            },
            body: body.into(),
        }
    }
}

impl Display for SequencedData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sequenced Data 0x{:02x?}", self.body)
    }
}

impl Default for SequencedData {
    fn default() -> Self {
        SequencedData::new(b"test SequencedData body")
    }
}

#[derive(ByteSerializeStack, ByteDeserializeBytes, PartialEq, Debug)]
pub struct SequencedData1 {
    header: SequencedDataHeader,
    body: Bytes,
}
impl SequencedData1 {
    pub fn new(body: Bytes) -> Self {
        SequencedData1 {
            header: SequencedDataHeader {
                packet_length: (body.len() + PacketTypeSequenceData::byte_size()) as u16,
                packet_type: PacketTypeSequenceData::default(),
            },
            body,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::des_bytes::from_bytes;
    use log::info;

    #[test]
    fn test_sequenced_data_bytes() {
        setup::log::configure();

        let msg_inp = SequencedData1::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10].to_vec().into());
        // info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.byte_len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: SequencedData1 = from_bytes(ser.as_slice().to_vec().into()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }

    #[test]
    fn test_sequenced_data_vec() {
        setup::log::configure();

        let msg_inp = SequencedData::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.byte_len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: SequencedData = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }

    #[test]
    fn test_panic() {
        setup::log::configure();
        let bytes = Bytes::from_static(b"1234");
        let slice = &b"1234"[..];
        let x = &slice[4..6];
    }
}
