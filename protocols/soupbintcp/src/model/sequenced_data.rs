use std::fmt::Debug;

use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

use super::types::PacketTypeSequencedData;

pub const SEQUENCED_DATA_HEADER_BYTE_LEN: usize = 3;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct SPayloadHeader {
    pub packet_length: u16,
    pub packet_type: PacketTypeSequencedData,
}

impl SPayloadHeader {
    pub fn new(packet_length: u16) -> Self {
        SPayloadHeader {
            packet_length,
            packet_type: PacketTypeSequencedData::default(),
        }
    }
}

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct SPayload<Payload>
where 
    Payload: ByteSerializeStack + ByteDeserializeSlice<Payload> + ByteSerializedLenOf + PartialEq + Clone + Debug
{
    header: SPayloadHeader,
    #[byteserde(deplete ( header.packet_length as usize - 1 ))]
    body: Payload,
}
#[rustfmt::skip]
impl<Payload: ByteSerializeStack + ByteDeserializeSlice<Payload> + ByteSerializedLenOf + PartialEq + Clone + Debug> SPayload<Payload>
{
    pub fn new(body: Payload) -> SPayload<Payload> {
        let header = SPayloadHeader::new((body.byte_len() + 1) as u16);
        SPayload { header, body }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::payload::SamplePayload;
    use links_network_core::unittest::setup;
    use log::info;

    #[test]
    fn test_sequenced_data_header() {
        setup::log::configure();
        let msg_inp = SPayloadHeader::new(10);
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(SEQUENCED_DATA_HEADER_BYTE_LEN, ser.len());
        assert_eq!(SEQUENCED_DATA_HEADER_BYTE_LEN, msg_inp.byte_len());

        let msg_out: SPayloadHeader = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
    #[test]
    fn test_sequenced_data() {
        setup::log::configure();
        let expected_len = SEQUENCED_DATA_HEADER_BYTE_LEN + SamplePayload::default().byte_len();
        let msg_inp = SPayload::new(SamplePayload::default());
        info!("msg_inp:? {:?}", msg_inp);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);
        assert_eq!(expected_len, ser.len());
        assert_eq!(expected_len, msg_inp.byte_len());

        let msg_out: SPayload<SamplePayload> = from_slice(ser.as_slice()).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
