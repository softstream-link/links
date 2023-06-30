// use bytes::Bytes;
use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserializeBytes, ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf,
};
use std::fmt::{Debug, Display};

use super::{sample_payload::SamplePayload, types::PacketTypeSequenceData};

pub const SEQUENCED_DATA_HEADER_BYTE_LEN: usize = 3;

#[derive(
    ByteSerializeStack,
    ByteDeserializeSlice,
    ByteDeserializeBytes,
    ByteSerializedLenOf,
    PartialEq,
    Debug,
)]
#[byteserde(endian = "be")]
pub struct SequencedDataHeader {
    pub packet_length: u16,
    pub packet_type: PacketTypeSequenceData,
}

impl SequencedDataHeader {
    pub fn new(packet_length: u16) -> Self {
        SequencedDataHeader {
            packet_length,
            packet_type: PacketTypeSequenceData::default(),
        }
    }
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq,
{
    header: SequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: T,
}
impl<T> SequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Debug,
{
    pub fn new(body: T) -> SequencedData<T> {
        let header = SequencedDataHeader::new((body.byte_len() + 1) as u16);
        SequencedData { header, body }
    }
}

impl Default for SequencedData<SamplePayload> {
    fn default() -> Self {
        SequencedData::new(SamplePayload::default())
    }
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedDataVec {
    header: SequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: Vec<u8>,
}

impl SequencedDataVec {
    pub fn new(body: &[u8]) -> Self {
        SequencedDataVec {
            header: SequencedDataHeader {
                packet_length: (body.len() + PacketTypeSequenceData::byte_size()) as u16,
                packet_type: PacketTypeSequenceData::default(),
            },
            body: body.into(),
        }
    }
}

impl Display for SequencedDataVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sequenced Data 0x{:02x?}", self.body)
    }
}

impl Default for SequencedDataVec {
    fn default() -> Self {
        SequencedDataVec::new(b"test SequencedData body")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{model::sample_payload::SamplePayload, unittest::setup};
    use log::info;

    #[test]
    fn test_sequenced_data_header() {
        setup::log::configure();
        let msg_inp = SequencedDataHeader::new(10);
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(SEQUENCED_DATA_HEADER_BYTE_LEN, ser.len());
        assert_eq!(SEQUENCED_DATA_HEADER_BYTE_LEN, msg_inp.byte_len());

        let msg_out: SequencedDataHeader = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
    #[test]
    fn test_sequenced_data() {
        setup::log::configure();
        let expected_len = SEQUENCED_DATA_HEADER_BYTE_LEN + SamplePayload::default().byte_len();
        let msg_inp = SequencedData::default();
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(expected_len, ser.len());
        assert_eq!(expected_len, msg_inp.byte_len());

        let msg_out: SequencedData<SamplePayload> = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }

    #[test]
    fn test_sequenced_data_vec() {
        setup::log::configure();

        let msg_inp = SequencedDataVec::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.byte_len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: SequencedDataVec = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
