pub mod price;
use byteserde::ser::ByteSerializeHeap;
use byteserde_types::prelude::*;
use byteserde_types::string_ascii_fixed;
// const char ascii
pub type PacketTypeEnterOrder = ConstCharAscii<b'O'>;

// string ascii fixed

string_ascii_fixed!(Symbol, 9, b' ', true);
string_ascii_fixed!(CltOrderId, 14, b' ', true);


pub enum Side {
    Buy,
    Sell,
    SellShort,
    SellShortExempt,
    Invalid,
}
impl From<&CharAscii> for Side {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'B' => Side::Buy,
            b'S' => Side::Sell,
            b'T' => Side::SellShort,
            b'U' => Side::SellShortExempt,
            _ => Side::Invalid,
        }
    }
}
impl From<Side> for CharAscii {
    fn from(value: Side) -> Self {
        match value {
            Side::Buy => b'B'.into(),
            Side::Sell => b'S'.into(),
            Side::SellShort => b'T'.into(),
            Side::SellShortExempt => b'U'.into(),
            Side::Invalid => panic!("Invalid side"),
        }
    }
}
pub enum TimeInForce {
    MarketHours,
    IOC, // imediate or cancel
    GoodTillExtendedHours,
    GoodTillTriggered, //expire time needs to be specified
    AfterHours,
    Invalid,
}
impl From<&CharAscii> for TimeInForce {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'0' => TimeInForce::MarketHours,
            b'3' => TimeInForce::IOC,
            b'5' => TimeInForce::GoodTillExtendedHours,
            b'6' => TimeInForce::GoodTillTriggered,
            b'E' => TimeInForce::AfterHours,
            _ => TimeInForce::Invalid,
        }
    }
}
impl From<TimeInForce> for CharAscii {
    fn from(value: TimeInForce) -> Self {
        match value {
            TimeInForce::MarketHours => b'0'.into(),
            TimeInForce::IOC => b'3'.into(),
            TimeInForce::GoodTillExtendedHours => b'5'.into(),
            TimeInForce::GoodTillTriggered => b'6'.into(),
            TimeInForce::AfterHours => b'E'.into(),
            TimeInForce::Invalid => panic!("Invalid time in force"),
        }
    }
}
// trait ByteCode {
//     fn code(&self) -> u8;
// }
pub enum Display {
    Visible,
    Hidden,
    Atttributable,
    Invalid,
}

// TODO what would serializer look like and what woul be the cost?
impl ByteSerializeHeap for Display {
    fn byte_serialize_heap(
        &self,
        ser: &mut byteserde::ser::ByteSerializerHeap,
    ) -> byteserde::prelude::Result<()> {
        match self {
            Display::Visible => ser.serialize_bytes_slice(&b"Y"[..])?,
            Display::Hidden => ser.serialize_bytes_slice(&b"N"[..])?,
            Display::Atttributable => ser.serialize_bytes_slice(&b"A"[..])?,
            Display::Invalid => panic!("Invalid display"),
        };
        Ok(())
    }
}
impl From<&CharAscii> for Display {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'Y' => Display::Visible,
            b'N' => Display::Hidden,
            b'A' => Display::Atttributable,
            _ => Display::Invalid,
        }
    }
}
impl From<Display> for CharAscii {
    fn from(value: Display) -> Self {
        match value {
            Display::Visible => b'Y'.into(),
            Display::Hidden => b'N'.into(),
            Display::Atttributable => b'A'.into(),
            Display::Invalid => panic!("Invalid display"),
        }
    }
}

pub enum Capacity {
    Agency,
    Principal,
    RisklessPrincipal,
    Other,
    Invalid,
}
impl From<&CharAscii> for Capacity {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'A' => Capacity::Agency,
            b'P' => Capacity::Principal,
            b'R' => Capacity::RisklessPrincipal,
            b'O' => Capacity::Other,
            _ => Capacity::Invalid,
        }
    }
}
impl From<Capacity> for CharAscii {
    fn from(value: Capacity) -> Self {
        match value {
            Capacity::Agency => b'A'.into(),
            Capacity::Principal => b'P'.into(),
            Capacity::RisklessPrincipal => b'R'.into(),
            Capacity::Other => b'O'.into(),
            Capacity::Invalid => panic!("Invalid capacity"),
        }
    }
}
pub enum IntMktSweepEligibility {
    Eligible,
    NotEligible,
    Invalid,
}
impl From<&CharAscii> for IntMktSweepEligibility {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'Y' => IntMktSweepEligibility::Eligible,
            b'N' => IntMktSweepEligibility::NotEligible,
            _ => IntMktSweepEligibility::Invalid,
        }
    }
}
impl From<IntMktSweepEligibility> for CharAscii {
    fn from(value: IntMktSweepEligibility) -> Self {
        match value {
            IntMktSweepEligibility::Eligible => b'Y'.into(),
            IntMktSweepEligibility::NotEligible => b'N'.into(),
            IntMktSweepEligibility::Invalid => panic!("Invalid int mkt sweep eligibility"),
        }
    }
}

pub enum CrossType {
    ContinuousMarket,
    OpeningCross,
    ClosingCross,
    HaltIPO,
    Supplemental,
    Retail,
    ExtendedLife,
    AfterHoursClose,
    Invalid,
}

impl From<&CharAscii> for CrossType {
    fn from(value: &CharAscii) -> Self {
        match value.char() {
            b'N' => CrossType::ContinuousMarket,
            b'O' => CrossType::OpeningCross,
            b'C' => CrossType::ClosingCross,
            b'H' => CrossType::HaltIPO,
            b'S' => CrossType::Supplemental,
            b'R' => CrossType::Retail,
            b'E' => CrossType::ExtendedLife,
            b'A' => CrossType::AfterHoursClose,
            _ => CrossType::Invalid,
        }
    }
}

impl From<CrossType> for CharAscii {
    fn from(value: CrossType) -> Self {
        match value {
            CrossType::ContinuousMarket => b'N'.into(),
            CrossType::OpeningCross => b'O'.into(),
            CrossType::ClosingCross => b'C'.into(),
            CrossType::HaltIPO => b'H'.into(),
            CrossType::Supplemental => b'S'.into(),
            CrossType::Retail => b'R'.into(),
            CrossType::ExtendedLife => b'E'.into(),
            CrossType::AfterHoursClose => b'A'.into(),
            CrossType::Invalid => panic!("Invalid cross type"),
        }
    }
}
