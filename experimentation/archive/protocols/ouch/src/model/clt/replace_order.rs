use crate::prelude::*;
use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct ReplaceOrder {
    packet_type: PacketTypeReplaceOrder,
    pub orig_user_ref_number: UserRefNumber,
    pub user_ref_number: UserRefNumber,
    pub quantity: Quantity,
    pub price: Price,
    pub time_in_force: TimeInForce,
    pub display: Display,
    pub int_mkt_sweep_eligibility: IntMktSweepEligibility,
    pub clt_order_id: CltOrderId,
    #[byteserde(replace( appendages.byte_len() ))]
    appendage_length: u16,
    #[byteserde(deplete(appendage_length))]
    pub appendages: OptionalAppendage,
}
impl CancelableOrder for ReplaceOrder {
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
impl From<&EnterOrder> for ReplaceOrder {
    fn from(enter_order: &EnterOrder) -> Self {
        Self {
            packet_type: PacketTypeReplaceOrder::default(),
            orig_user_ref_number: enter_order.user_ref_number,
            user_ref_number: UserRefNumber::default(), // default place holder, has to be replaced
            quantity: enter_order.quantity,
            price: enter_order.price,
            time_in_force: enter_order.time_in_force,
            display: enter_order.display,
            int_mkt_sweep_eligibility: enter_order.int_mkt_sweep_eligibility,
            clt_order_id: CltOrderId::default(), // default place holder, has to be replaced
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
        let msg_inp = ReplaceOrder::from(&EnterOrder::default());

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: ReplaceOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
