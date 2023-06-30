use crate::model::types::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct ReplaceOrder {
    packet_type: PacketTypeReplaceOrder,
    orig_user_ref_number: OriginalUserRefNumber,
    user_ref_number: UserRefNumber,
    quantity: Quantity,
    price: Price,
    time_in_force: TimeInForce,
    display: Display,
    int_mkt_sweep_eligibility: IntMktSweepEligibility,
    clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    appendages: OptionalAppendage,
}

impl Default for ReplaceOrder {
    fn default() -> Self {
        let appendages = OptionalAppendage {
            min_qty: Some(TagValueElement::<MinQty>::new(100.into())),
            ..Default::default()
        };
        Self {
            packet_type: PacketTypeReplaceOrder::default(),
            orig_user_ref_number: 2.into(),
            user_ref_number: 1.into(),
            quantity: 100.into(),
            price: 100.1234_f64.into(),
            time_in_force: TimeInForceEnum::MarketHours.into(),
            display: DisplayEnum::Visible.into(),
            int_mkt_sweep_eligibility: IntMktSweepEligibilityEnum::Eligible.into(),
            clt_order_id: b"DUMMY_CLT_ORDER_#1".as_slice().into(),
            appendage_length: appendages.byte_len() as u16,
            appendages,
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;

    use log::info;

    #[test]
    fn test_replace_order() {
        setup::log::configure();
        let msg_inp = ReplaceOrder::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: ReplaceOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
