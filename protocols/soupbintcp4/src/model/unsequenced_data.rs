use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt;

use super::payload::SamplePayload;
use super::types::PacketTypeUnsequencedData;

pub const UNSEQUENCED_DATA_BYTE_LEN: usize = 3;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(endian = "be")]
pub struct UnsequencedDataHeader {
    packet_length: u16,
    packet_type: PacketTypeUnsequencedData,
}
impl UnsequencedDataHeader {
    pub fn new(packet_length: u16) -> Self {
        UnsequencedDataHeader {
            packet_length,
            packet_type: PacketTypeUnsequencedData::default(),
        }
    }
}

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
pub struct UnsequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug,
{
    header: UnsequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: T,
}
#[rustfmt::skip]
impl<T> UnsequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug,
{
    pub fn new(body: T) -> UnsequencedData<T> {
        let header = UnsequencedDataHeader::new((body.byte_len() + 1) as u16);
        UnsequencedData { header, body }
    }
}

impl Default for UnsequencedData<SamplePayload> {
    fn default() -> Self {
        UnsequencedData::new(SamplePayload::default())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::payload::SamplePayload;
    use links_testing::unittest::setup;
    use log::info;

    #[test]
    fn test_unsequenced_data_header() {
        setup::log::configure();

        let msg_inp = UnsequencedDataHeader::new(10);
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(UNSEQUENCED_DATA_BYTE_LEN, ser.len());
        assert_eq!(UNSEQUENCED_DATA_BYTE_LEN, msg_inp.byte_len());

        let msg_out: UnsequencedDataHeader = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }

    #[test]
    fn test_unsequenced_data() {
        setup::log::configure();
        let expected_len = UNSEQUENCED_DATA_BYTE_LEN + SamplePayload::default().byte_len();
        let msg_inp = UnsequencedData::default();
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(expected_len, ser.len());
        assert_eq!(expected_len, msg_inp.byte_len());

        let msg_out: UnsequencedData<SamplePayload> = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
