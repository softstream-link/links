use byteserde_types::prelude::*;

pub type PacketTypeClientHeartbeat = ConstCharAscii<b'R'>;
pub type PacketTypeServerHeartbeat = ConstCharAscii<b'H'>;
pub type PacketTypeDebug = ConstCharAscii<b'+'>;
pub type PacketTypeEndOfSession = ConstCharAscii<b'Z'>;

pub type PacketTypeLoginAccepted = ConstCharAscii<b'A'>;
pub type PacketTypeLoginRejected = ConstCharAscii<b'J'>;
pub type PacketTypeLoginRequest = ConstCharAscii<b'L'>;
pub type PacketTypeLogoutRequest = ConstCharAscii<b'O'>;

pub type PacketTypeUnsequenceData = ConstCharAscii<b'U'>;
pub type PacketTypeSequenceData = ConstCharAscii<b'S'>;

const S: u8 = b' ';
const R: bool = true;

pub type SessionId = StringAsciiFixed<10, S, R>;
pub type SequenceNumber = StringAsciiFixed<20, S, R>;
pub type TimeoutMs = StringAsciiFixed<5, S, R>;


pub type UserName = StringAsciiFixed<6, S, R>;
pub type Password = StringAsciiFixed<10, S, R>;