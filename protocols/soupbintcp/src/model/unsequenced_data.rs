use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use std::fmt;

use crate::prelude::Nil;

use super::payload::SamplePayload;
use super::types::PacketTypeUnsequencedData;

pub const UNSEQUENCED_DATA_BYTE_LEN: usize = 3;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, fmt::Debug)]
#[byteserde(endian = "be")]
pub struct UPayloadHeader {
    packet_length: u16,
    packet_type: PacketTypeUnsequencedData,
}
impl UPayloadHeader {
    #[inline]
    pub fn new(packet_length: u16) -> Self {
        UPayloadHeader {
            packet_length,
            packet_type: PacketTypeUnsequencedData::default(),
        }
    }
}

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
pub struct UPayload<Payload>
where
    Payload: ByteSerializeStack + ByteDeserializeSlice<Payload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug,
{
    header: UPayloadHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    pub body: Payload,
}
#[rustfmt::skip]
impl<Payload> UPayload<Payload>
where
    Payload: ByteSerializeStack + ByteDeserializeSlice<Payload> + ByteSerializedLenOf + PartialEq + Clone + fmt::Debug,
{
    #[inline]
    pub fn new(body: Payload) -> UPayload<Payload> {
        let header = UPayloadHeader::new((body.byte_len() + 1) as u16);
        UPayload { header, body }
    }
}

impl Default for UPayload<SamplePayload> {
    fn default() -> Self {
        UPayload::new(SamplePayload::default())
    }
}
impl Default for UPayload<Nil>{
    fn default() -> Self {
        UPayload::new(Nil)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::payload::SamplePayload;
    use links_core::unittest::setup;
    use log::info;

    #[test]
    fn test_unsequenced_data_header() {
        setup::log::configure();

        let msg_inp = UPayloadHeader::new(10);
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(UNSEQUENCED_DATA_BYTE_LEN, ser.len());
        assert_eq!(UNSEQUENCED_DATA_BYTE_LEN, msg_inp.byte_len());

        let msg_out: UPayloadHeader = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }

    #[test]
    fn test_unsequenced_data() {
        setup::log::configure();
        let expected_len = UNSEQUENCED_DATA_BYTE_LEN + SamplePayload::default().byte_len();
        let msg_inp = UPayload::default();
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);
        assert_eq!(expected_len, ser.len());
        assert_eq!(expected_len, msg_inp.byte_len());

        let msg_out: UPayload<SamplePayload> = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
