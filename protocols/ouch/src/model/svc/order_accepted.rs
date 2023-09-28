use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderAccepted {
    packet_type: PacketTypeOrderAccepted,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub side: Side,
    pub quantity: Quantity,
    pub symbol: Symbol,
    pub price: Price,
    pub time_in_force: TimeInForce,
    pub display: Display,
    
    pub order_reference_number: OrderReferenceNumber, // Venue assigned

    pub capacity: Capacity,
    pub int_mkt_sweep_eligibility: IntMktSweepEligibility,
    pub cross_type: CrossType,

    pub order_state: OrderState, // Venue assigned

    pub clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    pub appendages: OptionalAppendage,
}

impl From<&EnterOrder> for OrderAccepted {
    fn from(enter_order: &EnterOrder) -> Self {
        Self {
            packet_type: PacketTypeOrderAccepted::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: enter_order.user_ref_number,
            side: enter_order.side,
            quantity: enter_order.quantity,
            symbol: enter_order.symbol,
            price: enter_order.price,
            time_in_force: enter_order.time_in_force,
            display: enter_order.display,

            order_reference_number: OrderReferenceNumber::default(), // Venue assigned

            capacity: enter_order.capacity,
            int_mkt_sweep_eligibility: enter_order.int_mkt_sweep_eligibility,
            cross_type: enter_order.cross_type,

            order_state: OrderState::live(), // Venue assigned

            clt_order_id: enter_order.clt_order_id,
            appendage_length: enter_order.appendages.byte_len() as u16,
            appendages: enter_order.appendages,
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
        let msg_inp = OrderAccepted::from(&enter_order);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderAccepted = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
