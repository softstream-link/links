use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderRestated {
    packet_type: PacketTypeOrderRestated,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub reason: RestatedReason,
 
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    pub appendages: OptionalAppendage,
}

impl From<(&EnterOrder, RestatedReason)> for OrderRestated {
    fn from(value: (&EnterOrder, RestatedReason)) -> Self {
        let (ord, reason) = value;
        Self {
            packet_type: PacketTypeOrderRestated::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: ord.user_ref_number,
            reason,
            appendage_length: ord.appendages.byte_len() as u16,
            appendages: ord.appendages,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_network_core::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();

        let enter_order = EnterOrder::default();
        let msg_inp = OrderRestated::from((&enter_order, RestatedReason::refresh_of_display()));

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderRestated = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
