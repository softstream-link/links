use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct CancelPending {
    packet_type: PacketTypeCancelPending,

    pub timestamp: Timestamp, // Venue assigned

    pub user_ref_number: UserRefNumber,
    
}

impl<T> From<&T> for CancelPending
where
    T: CancelableOrder,
{
    fn from(ord: &T) -> Self {
        Self {
            packet_type: PacketTypeCancelPending::default(),

            timestamp: Timestamp::default(), // Venue assigned

            user_ref_number: ord.user_ref_number(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use links_core::unittest::setup;
    use byteserde::prelude::*;
    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();

        let enter_order = EnterOrder::default();
        let msg_inp = CancelPending::from(&enter_order);

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: CancelPending = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
