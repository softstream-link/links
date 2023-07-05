use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct CancelOrder {
    packet_type: PacketTypeCancelOrder,
    user_ref_number: UserRefNumber,
    quantity: Quantity,
}
pub trait CancelableOrder {
    fn user_ref_number(&self) -> UserRefNumber;
    fn quantity(&self) -> Quantity;
}
impl<T: CancelableOrder> From<&T> for CancelOrder {
    fn from(ord: &T) -> Self {
        Self {
            packet_type: PacketTypeCancelOrder::default(),
            user_ref_number: ord.user_ref_number().clone(),
            quantity: ord.quantity().clone(),
        }
    }
}
impl CancelOrder {
    pub fn new(user_ref_number: UserRefNumber, quantity: Quantity) -> Self {
        Self {
            packet_type: PacketTypeCancelOrder::default(),
            user_ref_number: user_ref_number,
            quantity: quantity,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup;
    use log::info;
    use byteserde::prelude::*;

    #[test]
    fn test_msg() {
        setup::log::configure();
        
        let msg_inp = CancelOrder::from(&EnterOrder::default());

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: CancelOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
