use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderReplaced {
    packet_type: PacketTypeOrderReplaced,
    
    timestamp: Timestamp, // Venue assigned
    
    orig_user_ref_number: UserRefNumber,
    user_ref_number: UserRefNumber,
    side: Side, // from original order chain
    quantity: Quantity,
    symbol: Symbol, // from original order chain
    price: Price,
    time_in_force: TimeInForce,
    display: Display,
    
    order_reference_number: OrderReferenceNumber, // Venue assigned

    capacity: Capacity, // from original order chain
    int_mkt_sweep_eligibility: IntMktSweepEligibility,
    cross_type: CrossType, // from original order chain

    order_state: OrderState, // Venue assigned

    clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    appendages: OptionalAppendage,
}
impl From<(&EnterOrder, &ReplaceOrder)> for OrderReplaced {
    fn from(value: (&EnterOrder, &ReplaceOrder)) -> Self {
        let (enter_order, replace_order) = value;
        OrderReplaced {
            packet_type: PacketTypeOrderReplaced::default(),

            timestamp: Timestamp::default(),                         // Venue assigned
            order_reference_number: OrderReferenceNumber::default(), // default placeholder must be replaced
            order_state: OrderState::live(),                         // Venue assigned

            orig_user_ref_number: replace_order.orig_user_ref_number,
            user_ref_number: enter_order.user_ref_number, // enter_order
            side: enter_order.side,                       // enter_order
            symbol: enter_order.symbol,                   // enter_order
            capacity: enter_order.capacity,               // enter_order
            cross_type: enter_order.cross_type,           // enter_order

            quantity: replace_order.quantity,
            price: replace_order.price,
            time_in_force: replace_order.time_in_force,
            display: replace_order.display,
            int_mkt_sweep_eligibility: replace_order.int_mkt_sweep_eligibility,

            clt_order_id: replace_order.clt_order_id,
            appendage_length: replace_order.appendages.byte_len() as u16,
            appendages: replace_order.appendages,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_core::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let enter_order = EnterOrder::default();
        let mut replace_order = ReplaceOrder::from(&enter_order);
        replace_order.quantity = Quantity::new(50);

        let msg_inp = OrderReplaced::from((&enter_order, &replace_order));

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderReplaced = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
