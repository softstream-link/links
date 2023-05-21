use crate::model::types::{price::Price, *};
use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use byteserde_types::prelude::*;




#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
struct EnterOrder {
    packet_type: PacketTypeEnterOrder,
    user_ref_number: u32,
    side: CharAscii,
    quantity: u32,
    symbol: Symbol,
    price: Price,
    time_in_force: CharAscii,
    display: CharAscii,
    capacity: CharAscii,
    int_mkt_sweep_eligibility: CharAscii,
    cross_type: CharAscii,
    clt_order_id: CltOrderId,
    appendage_size: u16,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_enter_order() {
        setup::log::configure();

        let msg_inp = EnterOrder {
            packet_type: PacketTypeEnterOrder::default(),
            user_ref_number: Default::default(),
            side: Side::Buy.into(),
            quantity: Default::default(),
            symbol: b"IBM".as_slice().into(),
            price: 100.6554_f64.into(),
            time_in_force: TimeInForce::MarketHours.into(),
            display: Display::Visible.into(),
            capacity: Capacity::Agency.into(),
            int_mkt_sweep_eligibility: IntMktSweepEligibility::Eligible.into(),
            cross_type: CrossType::ContinuousMarket.into(),
            clt_order_id: b"1A".as_slice().into(),
            appendage_size: Default::default(),
        };

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: EnterOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
