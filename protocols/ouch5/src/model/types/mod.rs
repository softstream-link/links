
pub use capacity::{Capacity, CapacityEnum};
pub use cross_type::{CrossType, CrossTypeEnum};
pub use display::{Display, DisplayEnum};
pub use int_mkt_sweep_eligibility::{IntMktSweepEligibility, IntMktSweepEligibilityEnum};
pub use price::Price;
pub use side::{Side, SideEnum};
pub use string_ascii_fixed::{CltOrderId, Symbol};
pub use time_in_force::{TimeInForce, TimeInForceEnum};

use byteserde_derive::{ByteDeserialize, ByteEnumFromBinder, ByteSerializeStack};
use byteserde_types::{prelude::*, };
use byteserde_types::{char_ascii, string_ascii_fixed, u32_tuple, u64_tuple};

// const char ascii
pub type PacketTypeEnterOrder = ConstCharAscii<b'O'>;

// fixed strings
#[rustfmt::skip]
pub mod string_ascii_fixed{
    use super::*;
    string_ascii_fixed!(Symbol, 9, b' ', true, ByteSerializeStack, ByteDeserialize, PartialEq);
    string_ascii_fixed!(CltOrderId, 14, b' ', true, ByteSerializeStack, ByteDeserialize, PartialEq);
}

// enums
pub mod side {
    use super::*;
    char_ascii!(Side, ByteSerializeStack, ByteDeserialize, PartialEq);
    /// Helper for converting to and from [Side]
    #[derive(ByteEnumFromBinder)]
    #[byteserde(bind(Side), from(SideEnum))]
    pub enum SideEnum {
        #[byteserde(replace(Side(b'B')))]
        Buy,
        #[byteserde(replace(Side(b'S')))]
        Sell,
        #[byteserde(replace(Side(b'T')))]
        SellShort,
        #[byteserde(replace(Side(b'U')))]
        SellShortExempt,
    }
}
pub mod time_in_force {
    use super::*;
    char_ascii!(TimeInForce, ByteSerializeStack, ByteDeserialize, PartialEq);

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
pub mod display {
    use super::*;
    char_ascii!(Display, ByteSerializeStack, ByteDeserialize, PartialEq);

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
pub mod capacity {
    use super::*;
    char_ascii!(Capacity, ByteSerializeStack, ByteDeserialize, PartialEq);

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
pub mod int_mkt_sweep_eligibility {
    use super::*;
    char_ascii!(
        IntMktSweepEligibility,
        ByteSerializeStack,
        ByteDeserialize,
        PartialEq
    );

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
pub mod cross_type {
    use super::*;
    char_ascii!(CrossType, ByteSerializeStack, ByteDeserialize, PartialEq);

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
u32_tuple!(UserRefNumber, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug, Default);
u32_tuple!(Quantity, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug, Default);

pub mod price {
    use super::*; 
    u64_tuple!(Price, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug, Default);
    impl From<f64> for Price {
        fn from(f: f64) -> Self {
            Price((f * 10000.0) as u64)
        }
    }
}

