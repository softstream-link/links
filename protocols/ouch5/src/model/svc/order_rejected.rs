use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct OrderRejected {
    packet_type: PacketTypeOrderRejected,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    pub reason: RejectReason,
    pub clt_order_id: CltOrderId,
    
}

impl<T> From<(&T, RejectReason)> for OrderRejected
where
    T: CancelableOrder,
{
    fn from(value: (&T, RejectReason)) -> Self {
        let (ord, reason) = value;
        Self {            
            packet_type: PacketTypeOrderRejected::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: ord.user_ref_number(),
            reason,
            clt_order_id: ord.cl_ord_id(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_testing::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();

        let enter_order = EnterOrder::default();
        let msg_inp = OrderRejected::from((&enter_order, RejectReason::quote_unavailable()));

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: OrderRejected = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
