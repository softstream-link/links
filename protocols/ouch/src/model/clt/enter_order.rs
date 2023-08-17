use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct EnterOrder {
    packet_type: PacketTypeEnterOrder,
    pub user_ref_number: UserRefNumber,
    pub side: Side,
    pub quantity: Quantity,
    pub symbol: Symbol,
    pub price: Price,
    pub time_in_force: TimeInForce,
    pub display: Display,
    pub capacity: Capacity,
    pub int_mkt_sweep_eligibility: IntMktSweepEligibility,
    pub cross_type: CrossType,
    pub clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    pub appendages: OptionalAppendage,
}
impl EnterOrder {
    #[inline]
    pub fn new(
        user_ref_number: UserRefNumber,
        quantity: Quantity,
        symbol: Symbol,
        price: Price,
        time_in_force: TimeInForce,
        display: Display,
        capacity: Capacity,
        int_mkt_sweep_eligibility: IntMktSweepEligibility,
        cross_type: CrossType,
        clt_order_id: CltOrderId,
        appendages: OptionalAppendage,
    ) -> Self {
        Self {
            packet_type: PacketTypeEnterOrder::default(),
            user_ref_number,
            side: Side::buy(),
            quantity,
            symbol,
            price,
            time_in_force,
            display,
            capacity,
            int_mkt_sweep_eligibility,
            cross_type,
            clt_order_id,
            appendage_length: appendages.byte_len() as u16,
            appendages,
        }
    }
}

impl Default for EnterOrder {
    fn default() -> Self {
        let appendages = OptionalAppendage {
            customer_type: Some(TagValueElement::<CustomerType>::new(CustomerType::retail())),

            ..Default::default()
        };
        Self {
            packet_type: PacketTypeEnterOrder::default(),
            user_ref_number: UserRefNumberGenerator::default().next().unwrap(),
            side: Side::buy(),
            quantity: Quantity::from(100),
            symbol: Symbol::from(b"DUMMY".as_slice()),
            price: Price::from(1.2345),
            time_in_force: TimeInForce::market_hours(),
            display: Display::visible(),
            capacity: Capacity::agency(),
            int_mkt_sweep_eligibility: IntMktSweepEligibility::eligible(),
            cross_type: CrossType::continuous_market(),
            clt_order_id: CltOrderIdIterator::default().next().unwrap(),
            appendage_length: appendages.byte_len() as u16,
            appendages,
        }
    }
}
impl CancelableOrder for EnterOrder {
    fn user_ref_number(&self) -> UserRefNumber {
        self.user_ref_number
    }
    fn quantity(&self) -> Quantity {
        self.quantity
    }
    fn cl_ord_id(&self) -> CltOrderId {
        self.clt_order_id
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_testing::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let msg_inp = EnterOrder::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: EnterOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
