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

            user_ref_number: enter_order.user_ref_number.clone(),
            side: enter_order.side.clone(),
            quantity: enter_order.quantity.clone(),
            symbol: enter_order.symbol.clone(),
            price: enter_order.price.clone(),
            time_in_force: enter_order.time_in_force.clone(),
            display: enter_order.display.clone(),

            order_reference_number: OrderReferenceNumber::default(), // Venue assigned

            capacity: enter_order.capacity.clone(),
            int_mkt_sweep_eligibility: enter_order.int_mkt_sweep_eligibility.clone(),
            cross_type: enter_order.cross_type.clone(),

            order_state: OrderState::live(), // Venue assigned

            clt_order_id: enter_order.clt_order_id.clone(),
            appendage_length: enter_order.appendages.byte_len() as u16,
            appendages: enter_order.appendages.clone(),
        }
    }
}

impl Default for OrderAccepted {
    fn default() -> Self {
        OrderAccepted::from(&EnterOrder::default())
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
        let msg_inp = OrderAccepted::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderAccepted = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
