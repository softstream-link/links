use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct PriorityUpdate {
    packet_type: PacketTypePriorityUpdate,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub price: Price,
    pub display: Display,
    pub order_reference_number: OrderReferenceNumber, // Venue assigned
}

impl From<(&EnterOrder, OrderReferenceNumber)> for PriorityUpdate {
    fn from(value: (&EnterOrder, OrderReferenceNumber)) -> Self {
        let (ord, order_reference_number) = value;
        Self {
            packet_type: PacketTypePriorityUpdate::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: ord.user_ref_number,
            price: ord.price,
            display: ord.display,

            order_reference_number: order_reference_number, // Venue assigned
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();

        let enter_order = EnterOrder::default();
        let msg_inp = PriorityUpdate::from((&enter_order, OrderReferenceNumber::default()));

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: PriorityUpdate = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
