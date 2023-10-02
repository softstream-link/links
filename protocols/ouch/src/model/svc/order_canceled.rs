use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderCanceled {
    packet_type: PacketTypeOrderCanceled,
    
    timestamp: Timestamp, // Venue assigned
    
    orig_user_ref_number: UserRefNumber,
    user_ref_number: UserRefNumber,
    quantity: Quantity,
    reason: CancelReason,
}
impl From<(&EnterOrder, &CancelOrder)> for OrderCanceled {
    fn from(value: (&EnterOrder, &CancelOrder)) -> Self {
        let (enter_order, cancel_order) = value;
        Self {
            packet_type: PacketTypeOrderCanceled::default(),
            timestamp: Timestamp::default(),
            orig_user_ref_number: enter_order.user_ref_number,
            user_ref_number: cancel_order.user_ref_number,
            quantity: cancel_order.quantity,
            reason: CancelReason::user_requested(),
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use links_core::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let enter_order = EnterOrder::default();
        let mut cancel_order = CancelOrder::from(&enter_order);
        cancel_order.user_ref_number = UserRefNumber::new(enter_order.user_ref_number.value() + 1);

        let msg_inp = OrderCanceled::from((&enter_order, &cancel_order));

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderCanceled = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
