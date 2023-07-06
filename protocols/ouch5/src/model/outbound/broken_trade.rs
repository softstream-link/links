use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct BrokenTrade {
    packet_type: PacketTypeBrokenTrade,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub match_number: MatchNumber,
    pub reason: BrokenTradeReason,
    pub clt_order_id: CltOrderId,
}

impl<T> From<&T> for BrokenTrade
where
    T: CancelableOrder,
{
    fn from(ord: &T) -> Self {
        Self {
            packet_type: PacketTypeBrokenTrade::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: ord.user_ref_number(),
            match_number: MatchNumber::default(),
            reason: BrokenTradeReason::errorneous(),
            clt_order_id: ord.cl_ord_id(),
        }
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

        let enter_order = EnterOrder::default();
        let msg_inp = BrokenTrade::from(&enter_order);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: BrokenTrade = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
