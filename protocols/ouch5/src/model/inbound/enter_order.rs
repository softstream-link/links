use crate::model::{types::*, appendages::OptionalAppendage};
use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use byteserde::prelude::*;




#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
struct EnterOrder {
    packet_type: PacketTypeEnterOrder,
    user_ref_number: UserRefNumber,
    side: Side,
    quantity: Quantity,
    symbol: Symbol,
    price: Price,
    time_in_force: TimeInForce,
    display: Display,
    capacity: Capacity,
    int_mkt_sweep_eligibility: IntMktSweepEligibility,
    cross_type: CrossType,
    clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete( appendage_length ))]
    appendages: OptionalAppendage,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{unittest::setup, model::{types::TimeInForceEnum, appendages::{TagValueElement, CustomerType, CustomerTypeEnum}}};

    use log::info;

    #[test]
    fn test_enter_order() {
        setup::log::configure();
        let appendages = OptionalAppendage{
            customer_type: Some(TagValueElement::<CustomerType>::new(CustomerTypeEnum::Retail.into())),
            ..Default::default()
        };
        let msg_inp = EnterOrder {
            packet_type: PacketTypeEnterOrder::default(),
            user_ref_number: Default::default(),
            side: SideEnum::Sell.into(),
            quantity: Default::default(),
            symbol: b"IBM".as_slice().into(),
            price: 100.1234_f64.into(),
            time_in_force: TimeInForceEnum::MarketHours.into(),
            display: DisplayEnum::Visible.into(),
            capacity: CapacityEnum::Agency.into(),
            int_mkt_sweep_eligibility: IntMktSweepEligibilityEnum::Eligible.into(),
            cross_type: CrossTypeEnum::ContinuousMarket.into(),
            clt_order_id: b"1A".as_slice().into(),
            appendage_length: appendages.byte_len() as u16, 
            appendages,
        };



        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: EnterOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
