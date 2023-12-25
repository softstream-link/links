use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderExecuted {
    packet_type: PacketTypeOrderExecuted,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub quantity: Quantity,
    pub price: Price,
    pub liquidity_flag: LiquidityFlag,
    pub match_number: MatchNumber,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    pub appendages: OptionalAppendage,
}

impl From<&EnterOrder> for OrderExecuted {
    fn from(enter_order: &EnterOrder) -> Self {
        Self {
            packet_type: PacketTypeOrderExecuted::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: enter_order.user_ref_number,
            quantity: enter_order.quantity,
            price: enter_order.price,
            liquidity_flag: LiquidityFlag::added(),
            match_number: MatchNumber::default(),
            appendage_length: enter_order.appendages.byte_len() as u16,
            appendages: enter_order.appendages,
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
        let msg_inp = OrderExecuted::from(&enter_order);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderExecuted = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
