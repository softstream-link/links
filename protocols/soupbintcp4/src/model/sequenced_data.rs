use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt;

use super::{payload::SamplePayload, types::PacketTypeSequenceData};

pub const SEQUENCED_DATA_HEADER_BYTE_LEN: usize = 3;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
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

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedData<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug>
{
    header: SequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: T,
}
#[rustfmt::skip]
impl<T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug> SequencedData<T>
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{model::payload::SamplePayload, unittest::setup};
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
}
