pub use capacity::Capacity;
pub use cross_type::CrossType;
pub use display::Display;
pub use event_code::EventCode;
pub use int_mkt_sweep_eligibility::IntMktSweepEligibility;
pub use order_reference_number::OrderReferenceNumber;
pub use order_state::OrderState;
pub use price::Price;
pub use side::Side;
pub use time_in_force::TimeInForce;
pub use timestamp::Timestamp;

pub use super::appendages::*;
pub use clt_order_id::*;
pub use packet_types::*;
pub use qty::*;
pub use string_ascii_fixed::*;
pub use user_ref::*;

use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf, ByteSerializedSizeOf,
};
use byteserde_types::{char_ascii, string_ascii_fixed, u32_tuple, u64_tuple};

// const char ascii
#[rustfmt::skip]
pub mod packet_types{
    use super::*;
    use byteserde_types::const_char_ascii;
    // inbound
    const_char_ascii!(PacketTypeEnterOrder, b'O', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeReplaceOrder, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeCancelOrder, b'X', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeModifyOrder, b'M', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeAccountQueryRequest, b'Q', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    
    // outbound
    const_char_ascii!(PacketTypeSystemEvent, b'S', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeOrderAccepted, b'A', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    const_char_ascii!(PacketTypeOrderReplaced, b'U', ByteSerializeStack, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);

}
// fixed ascii strings
#[rustfmt::skip]
pub mod string_ascii_fixed{
    use super::*;
    string_ascii_fixed!(Symbol, 9, b' ', false, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
}

pub mod clt_order_id {
    use super::*;
    #[rustfmt::skip]
    string_ascii_fixed!(CltOrderId, 14, b' ', false, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl Default for CltOrderId {
        fn default() -> Self {
            Self::new(b"REPLACE_ME____".clone())
        }
    }
    impl From<u64> for CltOrderId {
        fn from(id: u64) -> Self {
            Self::from(format!("{}", id).as_str().as_bytes())
        }
    }
    pub struct CltOrderIdIterator {
        last: u64,
    }
    impl Default for CltOrderIdIterator {
        fn default() -> Self {
            Self { last: 0 }
        }
    }
    impl Iterator for CltOrderIdIterator {
        type Item = CltOrderId;
        fn next(&mut self) -> Option<Self::Item> {
            self.last += 1;
            Some(CltOrderId::from(self.last))
        }
    }
    #[cfg(test)]
    mod test {
        use log::info;
        use crate::unittest::setup;

        use super::*;

        #[test]
        fn test_clt_order_id_iterator() {
            setup::log::configure();
            let mut iter = CltOrderIdIterator { last: 0 };
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, CltOrderId::from(1));
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, CltOrderId::from(2));
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, CltOrderId::from(3));
        }
    }
}

// char ascii
#[rustfmt::skip]
pub mod side {
    use super::*;
    char_ascii!(Side, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl Side{
        pub fn buy() -> Self { Side(b'B') }
        pub fn sell() -> Self { Side(b'S') }
        pub fn sell_short() -> Self { Side(b'T') }
        pub fn sell_short_exempt() -> Self { Side(b'U') }
        pub fn is_buy(side: &Side) -> bool { Self::buy() == *side }
        pub fn is_sell(side: &Side) -> bool { Self::sell() == *side }
        pub fn is_sell_short(side: &Side) -> bool { Self::sell_short() == *side }
        pub fn is_sell_short_exempt(side: &Side) -> bool { Self::sell_short_exempt() == *side }
    }
}
#[rustfmt::skip]
pub mod time_in_force {
    use super::*;
    char_ascii!(TimeInForce, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl TimeInForce{
        pub fn market_hours() -> Self { TimeInForce(b'0') }
        pub fn immediate_or_cancel() -> Self { TimeInForce(b'3') }
        pub fn good_till_extended_hours() -> Self { TimeInForce(b'5') }
        pub fn good_till_triggered() -> Self { TimeInForce(b'6') }
        pub fn after_hours() -> Self { TimeInForce(b'E') }
        pub fn is_market_hours(tif: &TimeInForce) -> bool { Self::market_hours() == *tif }
        pub fn is_immediate_or_cancel(tif: &TimeInForce) -> bool { Self::immediate_or_cancel() == *tif }
        pub fn is_good_till_extended_hours(tif: &TimeInForce) -> bool { Self::good_till_extended_hours() == *tif }
        pub fn is_good_till_triggered(tif: &TimeInForce) -> bool { Self::good_till_triggered() == *tif }
        pub fn is_after_hours(tif: &TimeInForce) -> bool { Self::after_hours() == *tif }
    }
}
#[rustfmt::skip]
pub mod display {
    use super::*;
    char_ascii!(Display, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl Display {
        pub fn visible() -> Self { Display(b'Y') }
        pub fn hidden() -> Self { Display(b'N') }
        pub fn attributable() -> Self { Display(b'A') }
        pub fn conformant() -> Self { Display(b'Z') }
        pub fn is_visible(display: &Display) -> bool { Self::visible() == *display }
        pub fn is_hidden(display: &Display) -> bool { Self::hidden() == *display }
        pub fn is_attributable(display: &Display) -> bool { Self::attributable() == *display }
        pub fn is_conformant(display: &Display) -> bool { Self::conformant() == *display }
    }
}
#[rustfmt::skip]
pub mod capacity {
    use super::*;
    char_ascii!(Capacity, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl Capacity{
        pub fn agency() -> Self { Capacity(b'A') }
        pub fn principal() -> Self { Capacity(b'P') }
        pub fn riskless_principal() -> Self { Capacity(b'R') }
        pub fn other() -> Self { Capacity(b'O') }
        pub fn is_agency(capacity: &Capacity) -> bool { Self::agency() == *capacity }
        pub fn is_principal(capacity: &Capacity) -> bool { Self::principal() == *capacity }
        pub fn is_riskless_principal(capacity: &Capacity) -> bool { Self::riskless_principal() == *capacity }
        pub fn is_other(capacity: &Capacity) -> bool { Self::other() == *capacity }
    }
}
#[rustfmt::skip]
pub mod int_mkt_sweep_eligibility {
    use super::*;
    char_ascii!(IntMktSweepEligibility, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl IntMktSweepEligibility{
        pub fn eligible() -> Self { IntMktSweepEligibility(b'Y') }
        pub fn not_eligible() -> Self { IntMktSweepEligibility(b'N') }
        pub fn is_eligible(eligibility: &IntMktSweepEligibility) -> bool { Self::eligible() == *eligibility }
        pub fn is_not_eligible(eligibility: &IntMktSweepEligibility) -> bool { Self::not_eligible() == *eligibility }
    }
}
#[rustfmt::skip]
pub mod cross_type {
    use super::*;
    char_ascii!(CrossType, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
    impl CrossType{
        pub fn continuous_market() -> Self { CrossType(b'N') }
        pub fn opening_cross() -> Self { CrossType(b'O') }
        pub fn closing_cross() -> Self { CrossType(b'C') }
        pub fn halt_ipo() -> Self { CrossType(b'H') }
        pub fn supplemental() -> Self { CrossType(b'S') }
        pub fn retail() -> Self { CrossType(b'R') }
        pub fn extended_life() -> Self { CrossType(b'E') }
        pub fn after_hours_close() -> Self { CrossType(b'A') }
        pub fn is_continuous_market(cross_type: &CrossType) -> bool { Self::continuous_market() == *cross_type }
        pub fn is_opening_cross(cross_type: &CrossType) -> bool { Self::opening_cross() == *cross_type }
        pub fn is_closing_cross(cross_type: &CrossType) -> bool { Self::closing_cross() == *cross_type }
        pub fn is_halt_ipo(cross_type: &CrossType) -> bool { Self::halt_ipo() == *cross_type }
        pub fn is_supplemental(cross_type: &CrossType) -> bool { Self::supplemental() == *cross_type }
        pub fn is_retail(cross_type: &CrossType) -> bool { Self::retail() == *cross_type }
        pub fn is_extended_life(cross_type: &CrossType) -> bool { Self::extended_life() == *cross_type }
        pub fn is_after_hours_close(cross_type: &CrossType) -> bool { Self::after_hours_close() == *cross_type }
    }
}
#[rustfmt::skip]
pub mod event_code {
    use super::*;
    char_ascii!(EventCode, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    impl EventCode{
        pub fn start_of_day() -> Self { EventCode(b'S') }
        pub fn end_of_day() -> Self { EventCode(b'E') }
        pub fn is_startofday(side: &EventCode) -> bool { Self::start_of_day() == *side }
        pub fn is_endofday(side: &EventCode) -> bool { Self::end_of_day() == *side }
    }
}
#[rustfmt::skip]
pub mod order_state {
    use super::*;
    char_ascii!(OrderState, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
    impl OrderState{
        pub fn live() -> Self { OrderState(b'L') }
        pub fn dead() -> Self { OrderState(b'D') }
        pub fn is_live(side: &OrderState) -> bool { Self::live() == *side }
        pub fn is_dead(side: &OrderState) -> bool { Self::dead() == *side }
    }
}
// numerics
#[rustfmt::skip]
pub mod qty{
    use super::*;
    u32_tuple!(Quantity, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Debug, Default);
}
pub mod user_ref {
    use super::*;
    #[rustfmt::skip]
    u32_tuple!(UserRefNumber, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Debug, Default);

    #[rustfmt::skip]
    u32_tuple!(OriginalUserRefNumber, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Debug, Default);
    impl From<&UserRefNumber> for OriginalUserRefNumber {
        fn from(user_ref: &UserRefNumber) -> Self {
            OriginalUserRefNumber(user_ref.0.clone())
        }
    }
    pub struct UserRefNumberIterator {
        last: u32,
    }
    impl Default for UserRefNumberIterator {
        fn default() -> Self {
            UserRefNumberIterator { last: 0 }
        }
    }
    impl Iterator for UserRefNumberIterator {
        type Item = UserRefNumber;
        fn next(&mut self) -> Option<Self::Item> {
            if self.last == u32::MAX {
                None
            } else {
                self.last += 1;
                Some(UserRefNumber::new(self.last))
            }
        }
    }
    #[cfg(test)]
    mod test {
        use log::info;

        use crate::unittest::setup;

        use super::*;

        #[test]
        fn test_user_ref_number_iterator() {
            setup::log::configure();

            let mut iter = UserRefNumberIterator::default();
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, UserRefNumber::new(1));
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, UserRefNumber::new(2));
        }
    }
}

pub mod price {
    use std::fmt::Debug;

    use super::*;
    #[rustfmt::skip]
    u64_tuple!(Price, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Default);
    pub const PRICE_SCALE: f64 = 10000.0;
    impl From<f64> for Price {
        fn from(f: f64) -> Self {
            Price((f * PRICE_SCALE) as u64)
        }
    }
    impl Debug for Price {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple("Price")
                .field(&(self.0 as f64 / PRICE_SCALE))
                .finish()
        }
    }
}

pub mod timestamp {
    use chrono::{DateTime, Local, NaiveDateTime, Utc};

    use super::*;

    #[rustfmt::skip]
    u64_tuple!(Timestamp, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
    /// Converts into nanseconds from last midnight of a given [DateTime<Local>] and into a [Timestamp]
    impl From<DateTime<Local>> for Timestamp {
        fn from(dt: DateTime<Local>) -> Self {
            let naive_now = dt.naive_local();
            Timestamp::from(naive_now)
        }
    }
    impl From<DateTime<Utc>> for Timestamp {
        fn from(dt: DateTime<Utc>) -> Self {
            let naive_now = dt.naive_utc();
            Timestamp::from(naive_now)
        }
    }
    impl From<NaiveDateTime> for Timestamp {
        fn from(dt: NaiveDateTime) -> Self {
            let last_midnight = dt.date().and_hms_opt(0, 0, 0).unwrap();
            let duration = dt.signed_duration_since(last_midnight).to_std().unwrap();
            let nanosec_since_last_midnight = duration.as_nanos() as u64;
            Timestamp(nanosec_since_last_midnight)
        }
    }
    impl Default for Timestamp {
        fn default() -> Self {
            Timestamp::from(Local::now())
        }
    }

    #[cfg(test)]
    mod test {
        use crate::unittest::setup;
        use log::info;

        use super::*;
        #[test]
        fn test_timestamp() {
            setup::log::configure();

            // default
            let timestamp = Timestamp::default();
            info!("default timestamp: {:?}", timestamp);

            // from an arbitrary date
            let nanos_shift = 1000;
            let nanos_shift_past_midnight = Local::now()
                .date_naive()
                .and_hms_nano_opt(0, 0, 0, nanos_shift)
                .unwrap();

            info!("one_th_nano_past_midnight: {:?}", nanos_shift_past_midnight);
            let timestamp = Timestamp::from(nanos_shift_past_midnight);
            info!("nanos_shift: {}, timestamp: {:?}", nanos_shift, timestamp);
            assert_eq!(timestamp, Timestamp(nanos_shift as u64));
        }
    }
}

pub mod order_reference_number {
    use super::*;
    #[rustfmt::skip]
    u64_tuple!(OrderReferenceNumber, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Debug);

    impl Default for OrderReferenceNumber {
        fn default() -> Self {
            OrderReferenceNumber(0)
        }
    }
    pub struct OrderReferenceNumberIterator {
        last: u64,
    }
    impl Default for OrderReferenceNumberIterator {
        fn default() -> Self {
            OrderReferenceNumberIterator { last: 0 }
        }
    }
    impl Iterator for OrderReferenceNumberIterator {
        type Item = OrderReferenceNumber;
        fn next(&mut self) -> Option<Self::Item> {
            if self.last == u64::MAX {
                None
            } else {
                self.last += 1;
                Some(OrderReferenceNumber::new(self.last))
            }
        }
    }

    #[cfg(test)]
    mod test {
        use log::info;

        use super::*;
        use crate::unittest::setup;
        #[test]
        fn test_order_ref_number_iterator() {
            setup::log::configure();

            let mut iter = OrderReferenceNumberIterator::default();
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, OrderReferenceNumber::new(1));
            let next = iter.next().unwrap();
            info!("next: {:?}", next);
            assert_eq!(next, OrderReferenceNumber::new(2));
        }
    }
}
