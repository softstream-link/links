use byteserde_derive::{ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf};
use byteserde_types::string_ascii_fixed;

#[rustfmt::skip]
string_ascii_fixed!(Context1, 10, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
#[rustfmt::skip]
string_ascii_fixed!(Context2, 10, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
pub struct SamplePayload{
    pub context1: Context1,
    pub context2: Context2,
}

impl Default for SamplePayload{
    fn default() -> Self{
        Self{
            context1: b"10 char load".as_slice().into(),
            context2: b"hello world".as_slice().into(),
        }
    }
}

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
pub struct NoPayload;

#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
pub struct VecPayload{
    pub payload: Vec<u8>,
}