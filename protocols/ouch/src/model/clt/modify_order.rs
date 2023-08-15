use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct ModifyOrder {
    packet_type: PacketTypeModifyOrder,
    user_ref_number: UserRefNumber,
    side: Side,
    quantity: Quantity,
}

impl Default for ModifyOrder {
    fn default() -> Self {
        Self {
            packet_type: PacketTypeModifyOrder::default(),
            user_ref_number: 1.into(),
            side: Side::buy(),
            quantity: 100.into(),
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use links_testing::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let msg_inp = ModifyOrder::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: ModifyOrder = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
