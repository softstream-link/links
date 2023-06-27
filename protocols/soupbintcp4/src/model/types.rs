pub use field_types::*;
pub use packet_types::*;

use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserialize, ByteSerializeStack, ByteSerializedLenOf,
    ByteSerializedSizeOf,
};

#[rustfmt::skip]
pub mod packet_types{
    use super::*;
    use byteserde_types::const_char_ascii;
    const_char_ascii!(PacketTypeCltHeartbeat, b'R', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeSvcHeartbeat, b'H', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeDebug, b'+', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeEndOfSession, b'Z', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeLoginAccepted, b'A', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeLoginRejected, b'J', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeLoginRequest, b'L', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeLogoutRequest, b'O', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeSequenceData, b'S', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeUnsequenceData, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
}

#[rustfmt::skip]
pub mod field_types{
    use super::*;
    use byteserde_types::string_ascii_fixed;

    string_ascii_fixed!(SessionId, 10, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    
    string_ascii_fixed!(SequenceNumber, 20, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    impl From<u64> for SequenceNumber{ fn from(v: u64) -> Self { v.to_string().as_bytes().into()} }

    string_ascii_fixed!(TimeoutMs, 5, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    impl From<u16> for TimeoutMs{ fn from(v: u16) -> Self { v.to_string().as_bytes().into() } }
    
    string_ascii_fixed!(UserName, 6, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    string_ascii_fixed!(Password, 10, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);

}
