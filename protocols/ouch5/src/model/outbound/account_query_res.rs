use crate::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(endian = "be")]
pub struct AccountQueryResponse {
    pub packet_type: PacketTypeAccountQueryResponse,
    pub timestamp: Timestamp,
    pub next_user_ref_number: UserRefNumber,
}

impl Default for AccountQueryResponse {
    fn default() -> Self {
        Self {
            packet_type: PacketTypeAccountQueryResponse::default(),
            timestamp: Timestamp::default(),
            next_user_ref_number: UserRefNumber::default(),
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use byteserde::prelude::*;
    use crate::unittest::setup;

    use log::info;

    #[test]
    fn test_msg() {
        setup::log::configure();
        let msg_inp = AccountQueryResponse::default();

        let ser: ByteSerializerStack<128> = to_serializer_stack(&msg_inp).unwrap();
        info!("ser: {:#x}", ser);

        let msg_out: AccountQueryResponse = from_serializer_stack(&ser).unwrap();

        info!("msg_inp: {:?}", msg_inp);
        info!("msg_out: {:?}", msg_out);
        assert_eq!(msg_out, msg_inp);
    }
}
