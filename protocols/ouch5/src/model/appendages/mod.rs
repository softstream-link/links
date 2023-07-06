use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf, ByteSerializedSizeOf,
};
use byteserde_types::{char_ascii, i32_tuple, string_ascii_fixed, u16_tuple, u32_tuple, u64_tuple};
use std::fmt;

#[rustfmt::skip]
pub use optional_values::{
    secondary_ord_ref_num::*,
    firm::*,
    min_qty::*,
    customer_type::*, 
    max_floor::*, 
    price_type::*,
    peg_offset::*,
    discretion_price::*,
    discretion_price_type::*,
    discretion_peg_offset::*,
    post_only::*,
    random_reserves::*,
    route::*,
    expire_time::*,
    trade_now::*,
    handle_inst::*,
    bbo_weight_indicator::*,
    display_qty::*,
    display_price::*,
    group_id::*,
    shares_located::*,
};

pub trait OptionTag {
    fn tag() -> u8;
    fn tag_as_slice() -> &'static [u8];
}
macro_rules! option_tag {
    ($name:ident, $tag:literal) => {
        impl OptionTag for $name {
            fn tag() -> u8 {
                $tag
            }
            fn tag_as_slice() -> &'static [u8] {
                &[$tag]
            }
        }
    };
}

#[rustfmt::skip]
mod optional_values{
    use super::*;
    pub mod secondary_ord_ref_num{
        use super::*;
        u64_tuple!(SecondaryOrdRefNum, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(SecondaryOrdRefNum, 1);
    }
    pub mod firm {
        use super::*;
        string_ascii_fixed!(Firm, 4, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(Firm, 2);
    }
    pub mod min_qty {
        use super::*;
        u32_tuple!(MinQty, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(MinQty, 3);
    }
    pub mod customer_type{
        use super::*;
        char_ascii!(CustomerType, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(CustomerType, 4);
        
        impl CustomerType{
            pub fn retail() -> Self{ CustomerType(b'R') }
            pub fn non_retail_designated() -> Self{ CustomerType(b'N') }
            pub fn is_retail(other: CustomerType) -> bool{ CustomerType(b'R') == other}
            pub fn is_non_retail_designated(other: CustomerType) -> bool{ CustomerType(b'N') == other }
        }
      
    }
    pub mod max_floor{
        use super::*;
        u32_tuple!(MaxFloor, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(MaxFloor, 5);
    }
    pub mod price_type{
        use super::*;
        char_ascii!(PriceType, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(PriceType, 6);

        impl PriceType{
            pub fn limit() -> Self{ PriceType(b'L') }
            pub fn market_peg() -> Self{ PriceType(b'P') }
            pub fn mid_point_peg() -> Self{ PriceType(b'M') }
            pub fn primary_peg() -> Self{ PriceType(b'R') }
            pub fn market_maker_peg() -> Self{ PriceType(b'Q') }
            pub fn is_limit(other: PriceType) -> bool{ PriceType(b'L') == other }
            pub fn is_market_peg(other: PriceType) -> bool{ PriceType(b'P') == other }
            pub fn is_mid_point_peg(other: PriceType) -> bool{ PriceType(b'M') == other }
            pub fn is_primary_peg(other: PriceType) -> bool{ PriceType(b'R') == other }
            pub fn is_market_maker_peg(other: PriceType) -> bool{ PriceType(b'Q') == other }
        }
    }
    pub mod peg_offset{
        use super::*;
        i32_tuple!(PegOffset, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(PegOffset, 7);
    } 
    pub mod discretion_price{
        use super::*;
        u64_tuple!(DiscretionPrice, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(DiscretionPrice, 9);
    }
    pub mod discretion_price_type{
        use super::*;
        char_ascii!(DiscretionPriceType, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(DiscretionPriceType, 10);
        
        impl DiscretionPriceType{
            pub fn limit() -> Self{ DiscretionPriceType(b'L') }
            pub fn market_peg() -> Self{ DiscretionPriceType(b'P') }
            pub fn mid_point_peg() -> Self{ DiscretionPriceType(b'M') }
            pub fn primary_peg() -> Self{ DiscretionPriceType(b'R') }
            pub fn is_limit(other: DiscretionPriceType) -> bool{ DiscretionPriceType(b'L') == other }
            pub fn is_market_peg(other: DiscretionPriceType) -> bool{ DiscretionPriceType(b'P') == other }
            pub fn is_mid_point_peg(other: DiscretionPriceType) -> bool{ DiscretionPriceType(b'M') == other }
            pub fn is_primary_peg(other: DiscretionPriceType) -> bool{ DiscretionPriceType(b'R') == other }
        }
    }
    pub mod discretion_peg_offset{
        use super::*;
        i32_tuple!(DiscretionPegOffset, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(DiscretionPegOffset, 11);
    }
    pub mod post_only{
        use super::*;
        char_ascii!(PostOnly, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(PostOnly, 12);

        impl PostOnly{
            pub fn post_only() -> Self{ PostOnly(b'P') }
            pub fn no() -> Self{ PostOnly(b'N') }
            pub fn is_post_only(other: PostOnly) -> bool{ PostOnly(b'P') == other }
            pub fn is_no(other: PostOnly) -> bool{ PostOnly(b'N') == other }
        }
    }
    pub mod random_reserves{
        use super::*;
        u32_tuple!(RandomReserves, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(RandomReserves, 13);
    }
    pub mod route{
        use super::*;
        string_ascii_fixed!(Route, 4, b' ', true, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(Route, 14);
    }
    pub mod expire_time{
        use super::*;
        u32_tuple!(ExpireTime, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(ExpireTime, 15);
    }
    pub mod trade_now{
        use super::*;
        char_ascii!(TradeNow, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(TradeNow, 16);

        impl TradeNow{
            pub fn yes() -> Self{ TradeNow(b'Y') }
            pub fn no() -> Self{ TradeNow(b'N') }
            pub fn is_yes(other: TradeNow) -> bool{ TradeNow(b'Y') == other }
            pub fn is_no(other: TradeNow) -> bool{ TradeNow(b'N') == other }
        }
    }
    pub mod handle_inst{
        use super::*;
        char_ascii!(HandleInst, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(HandleInst, 17);

        impl HandleInst{
            pub fn imbalance_only() -> Self{ HandleInst(b'I') }
            pub fn retail_order_type_1() -> Self{ HandleInst(b'O') }
            pub fn retail_order_type_2() -> Self{ HandleInst(b'T') }
            pub fn retail_price_improvement() -> Self{ HandleInst(b'Q') }
            pub fn extended_life_continuous() -> Self{ HandleInst(b'B') }
            pub fn direct_listing_capital_raise() -> Self{ HandleInst(b'D') }
            pub fn hidden_price_improvement() -> Self{ HandleInst(b'R') }
            pub fn is_imbalance_only(other: HandleInst) -> bool{ HandleInst(b'I') == other }
            pub fn is_retail_order_type_1(other: HandleInst) -> bool{ HandleInst(b'O') == other }
            pub fn is_retail_order_type_2(other: HandleInst) -> bool{ HandleInst(b'T') == other }
            pub fn is_retail_price_improvement(other: HandleInst) -> bool{ HandleInst(b'Q') == other }
            pub fn is_extended_life_continuous(other: HandleInst) -> bool{ HandleInst(b'B') == other }
            pub fn is_direct_listing_capital_raise(other: HandleInst) -> bool{ HandleInst(b'D') == other }
            pub fn is_hidden_price_improvement(other: HandleInst) -> bool{ HandleInst(b'R') == other }
        }
    }
    pub mod bbo_weight_indicator{
        use super::*;
        char_ascii!(BBOWeightIndicator, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(BBOWeightIndicator, 18);

        impl BBOWeightIndicator{
            pub fn zero_point_2() -> Self{ BBOWeightIndicator(b'0') }
            pub fn point_2_one() -> Self{ BBOWeightIndicator(b'1') }
            pub fn one_two() -> Self{ BBOWeightIndicator(b'2') }
            pub fn two_above() -> Self{ BBOWeightIndicator(b'3') }
            pub fn unspecified() -> Self{ BBOWeightIndicator(b' ') }
            pub fn sets_qbbo_while_joining_nbbo() -> Self{ BBOWeightIndicator(b'S') }
            pub fn improves_nbbo_upon_entry() -> Self{ BBOWeightIndicator(b'N') }
            pub fn is_zero_point_2(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'0') == other }
            pub fn is_point_2_one(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'1') == other }
            pub fn is_one_two(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'2') == other }
            pub fn is_two_above(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'3') == other }
            pub fn is_unspecified(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b' ') == other }
            pub fn is_sets_qbbo_while_joining_nbbo(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'S') == other }
            pub fn is_improves_nbbo_upon_entry(other: BBOWeightIndicator) -> bool{ BBOWeightIndicator(b'N') == other }
        }
    }
    pub mod display_qty{
        use super::*;
        u32_tuple!(DisplayQty, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(DisplayQty, 22);
    }
    pub mod display_price{
        use super::*;
        u64_tuple!(DisplayPrice, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(DisplayPrice, 23);
    }
    pub mod group_id{
        use super::*;
        u16_tuple!(GroupId, "be", ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone, Copy);
        option_tag!(GroupId, 24);
    }
    pub mod shares_located{
        use super::*;
        char_ascii!(SharesLocated, ByteSerializeStack, ByteDeserializeSlice, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone, Copy);
        option_tag!(SharesLocated, 25);

        impl SharesLocated{
            pub fn yes() -> Self{ SharesLocated(b'Y') }
            pub fn no() -> Self{ SharesLocated(b'N') }
            pub fn is_yes(other: SharesLocated) -> bool{ SharesLocated(b'Y') == other }
            pub fn is_no(other: SharesLocated) -> bool{ SharesLocated(b'N') == other }
        }
    }
}

#[derive(
    ByteSerializeStack, ByteDeserializeSlice, PartialEq, ByteSerializedLenOf, Debug, Clone, Copy,
)]
pub struct TagValueElement<T>
where
    T: ByteSerializeStack
        + ByteDeserializeSlice<T>
        + ByteSerializedLenOf
        + Clone
        + Copy
        + fmt::Debug,
{
    length: u8,
    option_tag: u8,
    option_value: T,
}

#[rustfmt::skip]
impl<T> TagValueElement<T>
where
    T: ByteSerializeStack + ByteDeserializeSlice<T> + OptionTag + ByteSerializedLenOf + Clone + Copy + fmt::Debug,
{
    pub fn new(option_value: T) -> Self {
        TagValueElement {
            // remaining value of the TagValueElement
            length: 1 + std::mem::size_of::<T>() as u8, // NOTE: this only works because all types are tuples with single elements
            option_tag: T::tag(),
            option_value,
        }
    }
}

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Copy, Debug)]
#[byteserde(peek(1, 1))] // peek(start, len) -> peek one byte after skiping one
pub struct OptionalAppendage {
    #[byteserde(eq(SecondaryOrdRefNum::tag_as_slice()))]
    pub secondary_ord_ref_num: Option<TagValueElement<SecondaryOrdRefNum>>,

    #[byteserde(eq(Firm::tag_as_slice()))]
    pub firm: Option<TagValueElement<Firm>>,

    #[byteserde(eq(MinQty::tag_as_slice()))]
    pub min_qty: Option<TagValueElement<MinQty>>,

    #[byteserde(eq(CustomerType::tag_as_slice()))]
    pub customer_type: Option<TagValueElement<CustomerType>>,

    #[byteserde(eq(MaxFloor::tag_as_slice()))]
    pub max_floor: Option<TagValueElement<MaxFloor>>,

    #[byteserde(eq(PriceType::tag_as_slice()))]
    pub price_type: Option<TagValueElement<PriceType>>,

    #[byteserde(eq(PegOffset::tag_as_slice()))]
    pub peg_offset: Option<TagValueElement<PegOffset>>,

    #[byteserde(eq(DiscretionPrice::tag_as_slice()))]
    pub discretion_price: Option<TagValueElement<DiscretionPrice>>,

    #[byteserde(eq(DiscretionPriceType::tag_as_slice()))]
    pub discretion_price_type: Option<TagValueElement<DiscretionPriceType>>,

    #[byteserde(eq(DiscretionPegOffset::tag_as_slice()))]
    pub discretion_peg_offset: Option<TagValueElement<DiscretionPegOffset>>,

    #[byteserde(eq(PostOnly::tag_as_slice()))]
    pub post_only: Option<TagValueElement<PostOnly>>,

    #[byteserde(eq(RandomReserves::tag_as_slice()))]
    pub random_reserves: Option<TagValueElement<RandomReserves>>,

    #[byteserde(eq(Route::tag_as_slice()))]
    pub route: Option<TagValueElement<Route>>,

    #[byteserde(eq(ExpireTime::tag_as_slice()))]
    pub exprire_time: Option<TagValueElement<ExpireTime>>,

    #[byteserde(eq(TradeNow::tag_as_slice()))]
    pub trade_now: Option<TagValueElement<TradeNow>>,

    #[byteserde(eq(HandleInst::tag_as_slice()))]
    pub handle_inst: Option<TagValueElement<HandleInst>>,

    #[byteserde(eq(BBOWeightIndicator::tag_as_slice()))]
    pub bbo_weight_indicator: Option<TagValueElement<BBOWeightIndicator>>,

    #[byteserde(eq(DisplayQty::tag_as_slice()))]
    pub display_qty: Option<TagValueElement<DisplayQty>>,

    #[byteserde(eq(DisplayPrice::tag_as_slice()))]
    pub display_price: Option<TagValueElement<DisplayPrice>>,

    #[byteserde(eq(GroupId::tag_as_slice()))]
    pub group_id: Option<TagValueElement<GroupId>>,

    #[byteserde(eq(SharesLocated::tag_as_slice()))]
    pub shares_located: Option<TagValueElement<SharesLocated>>,
}

impl Default for OptionalAppendage {
    fn default() -> Self {
        OptionalAppendage {
            secondary_ord_ref_num: None,
            firm: None,
            min_qty: None,
            customer_type: None,
            max_floor: None,
            price_type: None,
            peg_offset: None,
            discretion_price: None,
            discretion_price_type: None,
            discretion_peg_offset: None,
            post_only: None,
            random_reserves: None,
            route: None,
            exprire_time: None,
            trade_now: None,
            handle_inst: None,
            bbo_weight_indicator: None,
            display_qty: None,
            display_price: None,
            group_id: None,
            shares_located: None,
        }
    }
}

#[test]
fn tag_value_elements() {
    use crate::unittest::setup;
    use log::info;
    setup::log::configure();

    let msg_sec_ord_ref = TagValueElement::<SecondaryOrdRefNum>::new(SecondaryOrdRefNum::new(1));
    let msg_firm = TagValueElement::<Firm>::new(Firm::new(*b"ABCD"));
    let msg_min_qty = TagValueElement::<MinQty>::new(MinQty::new(1));
    info!("msg_sec_ord_ref: \t{:?}", msg_sec_ord_ref);
    info!("msg_firm: \t{:?}", msg_firm);
    info!("msg_min_qty: \t{:?}", msg_min_qty);
    let inp_appendage = OptionalAppendage {
        secondary_ord_ref_num: Some(msg_sec_ord_ref.clone()),
        firm: Some(msg_firm.clone()),
        min_qty: Some(msg_min_qty.clone()),
        ..Default::default()
    };
    let _ = inp_appendage.clone(); // to ensure clone is propagated to all members

    let mut ser = ByteSerializerStack::<128>::default();
    ser.serialize(&inp_appendage).unwrap();
    info!("ser: {:#x}", ser);

    let mut des = ByteDeserializerSlice::new(ser.as_slice());
    let out_appendage = OptionalAppendage::byte_deserialize(&mut des).unwrap();
    info!("inp_appendage: {:?}", inp_appendage);
    info!("out_appendage: {:?}", out_appendage);
    assert_eq!(inp_appendage, out_appendage);
}
