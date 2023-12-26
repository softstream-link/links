use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderAiqCanceled {
    packet_type: PacketTypeOrderAiqCanceled,
    
    timestamp: Timestamp, // Venue assigned
    
    user_ref_number: UserRefNumber,
    decrement_shares: Quantity,
    reason: CancelReasonAiq,
    prevented_from_trading: Quantity,
    execution_price: Price,
    liquidity_flag: LiquidityFlag,
    aiq_strategy: AiqStrategy,


}
impl<T> From<&T> for OrderAiqCanceled
where T: CancelableOrder
{
    fn from(enter_order: &T) -> Self {
        Self {
            packet_type: PacketTypeOrderAiqCanceled::default(),
            timestamp: Timestamp::default(),
            user_ref_number: enter_order.user_ref_number(),
            decrement_shares: Quantity::default(),
            reason: CancelReasonAiq::default(),
            prevented_from_trading: Quantity::default(),
            execution_price: Price::default(),
            liquidity_flag: LiquidityFlag::added(),
            aiq_strategy: AiqStrategy::default(),
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_core::unittest::setup;
    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let enter_order = EnterOrder::default();

        let msg_inp = OrderAiqCanceled::from(&enter_order);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderAiqCanceled = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
