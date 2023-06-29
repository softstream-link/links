use byteserde::prelude::*;
use byteserde_derive::{ByteSerializeStack, ByteDeserializeSlice};

use crate::prelude::*;

#[derive(ByteSerializeStack, ByteDeserializeSlice, Debug, PartialEq)]
#[byteserde(peek(0, 1))]
pub enum Ouch5 {
    #[byteserde(eq(PacketTypeEnterOrder::as_slice()))]
    EntOrd(EnterOrder),
    #[byteserde(eq(PacketTypeReplaceOrder::as_slice()))]
    RepOrd(ReplaceOrder),
}
