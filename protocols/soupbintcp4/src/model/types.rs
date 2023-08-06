pub use field_types::*;
pub use packet_types::*;

use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf,
    ByteSerializedSizeOf,
};

#[rustfmt::skip]
pub mod packet_types{
    use super::*;
    use byteserde_types::const_char_ascii;
    const_char_ascii!(PacketTypeCltHeartbeat, b'R', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeSvcHeartbeat, b'H', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeDebug, b'+', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeEndOfSession, b'Z', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeLoginAccepted, b'A', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeLoginRejected, b'J', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeLoginRequest, b'L', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeLogoutRequest, b'O', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeSequencedData, b'S', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    const_char_ascii!(PacketTypeUnsequencedData, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
}

#[rustfmt::skip]
pub mod field_types{
    use super::*;
    use byteserde_types::string_ascii_fixed;

    string_ascii_fixed!(SessionId, 10, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    impl Default for SessionId{
        fn default() -> Self {
            b"#1".as_slice().into()
        }
    }

    string_ascii_fixed!(SequenceNumber, 20, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    impl From<u64> for SequenceNumber{ fn from(v: u64) -> Self { v.to_string().as_bytes().into()} }
    impl From<i32> for SequenceNumber{ fn from(v: i32) -> Self { if v <=0 { panic!("sequence number must be positive")} v.to_string().as_bytes().into() } }
    impl Default for SequenceNumber{
        fn default() -> Self {
            b"0".as_slice().into() // TODO check if 0 is acceptable
        }
    }

    string_ascii_fixed!(TimeoutMs, 5, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    impl From<u16> for TimeoutMs{ fn from(v: u16) -> Self { v.to_string().as_bytes().into() } }
    impl Default for TimeoutMs{
        fn default() -> Self {
            1000u16.into()
        }
    }
    string_ascii_fixed!(UserName, 6, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    string_ascii_fixed!(Password, 10, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);

}
