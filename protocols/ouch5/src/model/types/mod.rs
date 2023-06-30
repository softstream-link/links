pub use capacity::{Capacity, CapacityEnum};
pub use cross_type::{CrossType, CrossTypeEnum};
pub use display::{Display, DisplayEnum};
pub use int_mkt_sweep_eligibility::{IntMktSweepEligibility, IntMktSweepEligibilityEnum};
pub use price::Price;
pub use side::{Side, SideEnum};
pub use time_in_force::{TimeInForce, TimeInForceEnum};

pub use super::appendages::*;
pub use numerics::*;
pub use packet_types::*;
pub use string_ascii_fixed::*;

use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserializeSlice, ByteEnumFromBinder, ByteSerializeStack, ByteSerializedLenOf,
    ByteSerializedSizeOf,
};
use byteserde_types::{char_ascii, string_ascii_fixed, u32_tuple, u64_tuple};

// const char ascii
#[rustfmt::skip]
pub mod packet_types{
    use super::*;
    use byteserde_types::const_char_ascii;
    const_char_ascii!(PacketTypeEnterOrder, b'O', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    const_char_ascii!(PacketTypeReplaceOrder, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeDebug, b'+', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeEndOfSession, b'Z', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeLoginAccepted, b'A', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeLoginRejected, b'J', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeLoginRequest, b'L', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeLogoutRequest, b'O', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeSequenceData, b'S', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    // const_char_ascii!(PacketTypeUnsequenceData, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
}
// fixed strings
#[rustfmt::skip]
pub mod string_ascii_fixed{
    use super::*;
    string_ascii_fixed!(Symbol, 9, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
    string_ascii_fixed!(CltOrderId, 14, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);
}

#[rustfmt::skip]
pub mod side {
    use super::*;
    char_ascii!(Side, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    /// Helper for converting to and from [Side]
    pub enum SideEnum {
        Buy,
        Sell,
        SellShort,
        SellShortExempt,
        NotDefined(Side),
    }
    impl From<SideEnum> for Side {
        fn from(v: SideEnum) -> Self {
            match v {
                SideEnum::Buy => Side(b'B'),
                SideEnum::Sell => Side(b'S'),
                SideEnum::SellShort => Side(b'T'),
                SideEnum::SellShortExempt => Side(b'U'),
                SideEnum::NotDefined(s) => s,
            }
        }
    }
    impl From<Side> for SideEnum{
        fn from(v: Side) -> Self {
            match v {
                Side(b'B') => SideEnum::Buy,
                Side(b'S') => SideEnum::Sell,
                Side(b'T') => SideEnum::SellShort,
                Side(b'U') => SideEnum::SellShortExempt,
                _ => SideEnum::NotDefined(v),
            }
        }
    }

}
#[rustfmt::skip]
pub mod time_in_force {
    use super::*;
    char_ascii!(TimeInForce, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);

    /// Helper used for converting to and from [TimeInForce]
    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(TimeInForce), from(TimeInForceEnum))]
    pub enum TimeInForceEnum {
        #[byteserde(replace(TimeInForce(b'0')))]
        MarketHours,
        #[byteserde(replace(TimeInForce(b'3')))]
        ImmediateOrCancel,
        #[byteserde(replace(TimeInForce(b'5')))]
        GoodTillExtendedHours,
        #[byteserde(replace(TimeInForce(b'6')))]
        GoodTillTriggered,
        #[byteserde(replace(TimeInForce(b'E')))]
        AfterHours,
    }
}
#[rustfmt::skip]
pub mod display {
    use super::*;
    char_ascii!(Display, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);

    /// Helper used for converting to and from [Display]
    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(Display), from(DisplayEnum))]
    pub enum DisplayEnum {
        #[byteserde(replace(Display(b'Y')))]
        Visible,
        #[byteserde(replace(Display(b'N')))]
        Hidden,
        #[byteserde(replace(Display(b'A')))]
        Atttributable,
    }
}
#[rustfmt::skip]
pub mod capacity {
    use super::*;
    char_ascii!(Capacity, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);

    /// Helper used for converting to and from [Capacity]
    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(Capacity))]
    #[byteserde(from(Capacity))]
    #[byteserde(from(CapacityEnum))]
    pub enum CapacityEnum {
        #[byteserde(replace(Capacity(b'A')))]
        Agency,
        #[byteserde(replace(Capacity(b'P')))]
        Principal,
        #[byteserde(replace(Capacity(b'R')))]
        RisklessPrincipal,
        #[byteserde(replace(Capacity(b'O')))]
        Other,
    }
}
#[rustfmt::skip]
pub mod int_mkt_sweep_eligibility {
    use super::*;
    char_ascii!(IntMktSweepEligibility, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf,PartialEq);

    /// Helper used for converting to and from [IntMktSweepEligibility]
    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(IntMktSweepEligibility))]
    #[byteserde(from(IntMktSweepEligibility))]
    #[byteserde(from(IntMktSweepEligibilityEnum))]
    pub enum IntMktSweepEligibilityEnum {
        #[byteserde(replace(IntMktSweepEligibility(b'Y')))]
        Eligible,
        #[byteserde(replace(IntMktSweepEligibility(b'N')))]
        NotEligible,
    }
}
#[rustfmt::skip]
pub mod cross_type {
    use super::*;
    char_ascii!(CrossType, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq);

    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(CrossType))]
    #[byteserde(from(CrossType))]
    #[byteserde(from(CrossTypeEnum))]
    pub enum CrossTypeEnum {
        #[byteserde(replace(CrossType(b'N')))]
        ContinuousMarket,
        #[byteserde(replace(CrossType(b'O')))]
        OpeningCross,
        #[byteserde(replace(CrossType(b'C')))]
        ClosingCross,
        #[byteserde(replace(CrossType(b'H')))]
        HaltIPO,
        #[byteserde(replace(CrossType(b'S')))]
        Supplemental,
        #[byteserde(replace(CrossType(b'R')))]
        Retail,
        #[byteserde(replace(CrossType(b'E')))]
        ExtendedLife,
        #[byteserde(replace(CrossType(b'A')))]
        AfterHoursClose,
    }
}

// numerics
#[rustfmt::skip]
pub mod numerics{
    use super::*;
    u32_tuple!(UserRefNumber, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Default);
    u32_tuple!(OriginalUserRefNumber, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Default);
    u32_tuple!(Quantity, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Default);
}

pub mod price {
    use super::*;
    #[rustfmt::skip]
    u64_tuple!(Price, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Default);
    impl From<f64> for Price {
        fn from(f: f64) -> Self {
            Price((f * 10000.0) as u64)
        }
    }
}
