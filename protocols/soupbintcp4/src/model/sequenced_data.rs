use bytes::Bytes;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use std::fmt::Display;

use super::types::PacketTypeSequenceData;

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct SequencedDataHeader {
    packet_length: u16,
    packet_type: PacketTypeSequenceData,
}

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
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

impl Default for SequencedData{
    fn default() -> Self {
        SequencedData::new(b"test SequencedData body")
    }
}

// #[derive(Debug, PartialEq)]
// pub struct SequencedData2<'des> {
//     header: SequencedDataHeader,
//     body: &'des [u8],
// }

// impl<'des> SequencedData2<'des>{
//     pub fn new(body: &'des [u8]) -> Self {
//         SequencedData2 {
//             header: SequencedDataHeader {
//                 packet_length: (body.len() + PacketTypeSequenceData::byte_size()) as u16,
//                 packet_type: PacketTypeSequenceData::default(),
//             },
//             body,
//         }
//     }
// }

// impl<'des> ByteDeserialize<SequencedData2<'des>> for SequencedData2<'des> {
//     fn byte_deserialize(des: &mut ByteDeserializer) -> Result<SequencedData2<'des>> {
//         let header: SequencedDataHeader = des.deserialize()?;
//         let body = des.deserialize_bytes_slice(header.packet_length as usize - 1)?;
//         Ok(SequencedData2 { header, body })
//     }
// }
// impl<'des> ByteSerializeStack for SequencedData2<'des> {
//     fn byte_serialize_stack<const CAP: usize>(
//         &self,
//         ser: &mut ByteSerializerStack<CAP>,
//     ) -> Result<()> {
//         ser.serialize(&self.header)?;
//         ser.serialize_bytes_slice(self.body)?;
//         Ok(())
//     }
// }
#[derive(Debug, PartialEq)]
pub struct SequencedData1 {
    header: SequencedDataHeader,
    body: Bytes,
}
impl SequencedData1{
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

impl ByteSerializeStack for SequencedData1 {
    fn byte_serialize_stack<const CAP: usize>(
        &self,
        ser: &mut ByteSerializerStack<CAP>,
    ) -> Result<()> {
        self.header.byte_serialize_stack(ser)?;
        ser.serialize_bytes_slice(&self.body[..])?;
        Ok(())
    }
}

impl ByteDeserialize<SequencedData1> for SequencedData1{
    fn byte_deserialize(des: &mut ByteDeserializer) -> Result<SequencedData1> {
        let header: SequencedDataHeader = des.deserialize()?;
        let bytes: Vec<u8> = des.deserialize_take(header.packet_length as usize - 1)?;
        let body = bytes.into();
        Ok(SequencedData1{
            header,
            body,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;

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
    fn test_sequenced_data_bytes() {
        setup::log::configure();
        let body = Bytes::from_static(b"1234");
        let msg_inp = SequencedData1::new(body);
        // info!("msg_inp: {}", msg_inp);
        info!("msg_inp:? {:?}", msg_inp);
        assert_eq!(
            msg_inp.header.packet_length as usize,
            msg_inp.body.len() + msg_inp.header.packet_type.byte_len() as usize
        );

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:x}", ser);

        let msg_out: SequencedData1 = from_serializer_stack(&ser).unwrap();
        info!("msg_out:? {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
