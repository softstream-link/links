use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt::Display;

use super::types::PacketTypeUnsequenceData;

pub const UNSEQUENCED_DATA_BYTE_LEN: usize = 3;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct UnsequencedDataHeader {
    packet_length: u16,
    packet_type: PacketTypeUnsequenceData,
}
impl UnsequencedDataHeader {
    pub fn new(packet_length: u16) -> Self {
        UnsequencedDataHeader {
            packet_length,
            packet_type: PacketTypeUnsequenceData::default(),
        }
    }
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Debug)]
pub struct UnsequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + std::fmt::Debug,
{
    header: UnsequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: T,
}

impl<T> UnsequencedData<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Debug,
{
    pub fn new(body: T) -> UnsequencedData<T> {
        let header = UnsequencedDataHeader::new((body.byte_len() + 1) as u16);
        UnsequencedData { header, body }
    }
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, PartialEq, Debug)] // TODO ByteSerializedLenOf fails on this struct why?
pub struct UnsequencedDataVec {
    header: UnsequencedDataHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: Vec<u8>,
}

impl UnsequencedDataVec {
    pub fn new(body: &[u8]) -> Self {
        UnsequencedDataVec {
            header: UnsequencedDataHeader {
                packet_length: (body.len() + PacketTypeUnsequenceData::byte_size()) as u16,
                packet_type: PacketTypeUnsequenceData::default(),
            },
            body: body.into(),
        }
    }
}

impl Display for UnsequencedDataVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unsequenced Data 0x{:02x?}", self.body)
    }
}

impl Default for UnsequencedDataVec {
    fn default() -> Self {
        UnsequencedDataVec::new(b"test UnsequencedData body")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
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

        let msg_inp = UnsequencedDataVec::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.byte_len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: UnsequencedDataVec = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
