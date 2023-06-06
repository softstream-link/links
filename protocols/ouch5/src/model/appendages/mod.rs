use byteserde::prelude::*;
use byteserde_derive::{
    ByteDeserialize, ByteEnumFromBinder, ByteSerializeStack, ByteSerializedLenOf,
    ByteSerializedSizeOf,
};
use byteserde_types::{char_ascii, i32_tuple, string_ascii_fixed, u16_tuple, u32_tuple, u64_tuple};

#[rustfmt::skip]
pub use optional_value::{
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
mod optional_value{
    use super::*;
    pub mod secondary_ord_ref_num{
        use super::*;
        u64_tuple!(SecondaryOrdRefNum, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(SecondaryOrdRefNum, 1);
    }
    pub mod firm {
        use super::*;
        string_ascii_fixed!(Firm, 4, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(Firm, 2);
    }
    pub mod min_qty {
        use super::*;
        u32_tuple!(MinQty, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(MinQty, 3);
    }
    pub mod customer_type{
        use super::*;
        char_ascii!(CustomerType, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(CustomerType, 4);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(CustomerType), from(CustomerType), from(CustomerTypeEnum))]
        pub enum CustomerTypeEnum{
            #[byteserde(replace(CustomerType(b'R')))]
            Retail,
            #[byteserde(replace(CustomerType(b'N')))]
            NonRetailDesignated,
        }
    }
    pub mod max_floor{
        use super::*;
        u32_tuple!(MaxFloor, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(MaxFloor, 5);
    }
    pub mod price_type{
        use super::*;
        char_ascii!(PriceType, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(PriceType, 6);
        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(PriceType), from(PriceType), from(PriceTypeEnum))]
        pub enum PriceTypeEnum{
            #[byteserde(replace(PriceType(b'L')))]
            Limit,
            #[byteserde(replace(PriceType(b'P')))]
            MarketPeg,
            #[byteserde(replace(PriceType(b'M')))]
            MidPointPeg,
            #[byteserde(replace(PriceType(b'R')))]
            PrimaryPeg,
            #[byteserde(replace(PriceType(b'Q')))]
            MarketMakerPeg,
        }
    }
    pub mod peg_offset{
        use super::*;
        i32_tuple!(PegOffset, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(PegOffset, 7);
    } 
    pub mod discretion_price{
        use super::*;
        u64_tuple!(DiscretionPrice, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(DiscretionPrice, 9);
    }
    pub mod discretion_price_type{
        use super::*;
        char_ascii!(DiscretionPriceType, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(DiscretionPriceType, 10);
        
        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(DiscretionPriceType), from(DiscretionPriceTypeEnum), from(DiscretionPriceType))]
        pub enum DiscretionPriceTypeEnum{
            #[byteserde(replace(DiscretionPriceType(b'L')))]
            Limit,
            #[byteserde(replace(DiscretionPriceType(b'P')))]
            MarketPeg,
            #[byteserde(replace(DiscretionPriceType(b'M')))]
            MidPointPeg,
            #[byteserde(replace(DiscretionPriceType(b'R')))]
            PrimaryPeg,
        }
    }
    pub mod discretion_peg_offset{
        use super::*;
        i32_tuple!(DiscretionPegOffset, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(DiscretionPegOffset, 11);
    }
    pub mod post_only{
        use super::*;
        char_ascii!(PostOnly, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(PostOnly, 12);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(PostOnly), from(PostOnlyEnum), from(PostOnly))]
        pub enum PostOnlyEnum{
            #[byteserde(replace(PostOnly(b'P')))]
            PostOnly,
            #[byteserde(replace(PostOnly(b'N')))]
            No,
        }
    }
    pub mod random_reserves{
        use super::*;
        u32_tuple!(RandomReserves, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(RandomReserves, 13);
    }
    pub mod route{
        use super::*;
        string_ascii_fixed!(Route, 4, b' ', true, ByteSerializeStack, ByteDeserialize, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(Route, 14);
    }
    pub mod expire_time{
        use super::*;
        u32_tuple!(ExpireTime, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(ExpireTime, 15);
    }
    pub mod trade_now{
        use super::*;
        char_ascii!(TradeNow, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(TradeNow, 16);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(TradeNow), from(TradeNowEnum), from(TradeNow))]
        pub enum TradeNowEnum{
            #[byteserde(replace(TradeNow(b'Y')))]
            Yes,
            #[byteserde(replace(TradeNow(b'N')))]
            No,
        }
    }
    pub mod handle_inst{
        use super::*;
        char_ascii!(HandleInst, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(HandleInst, 17);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(HandleInst), from(HandleInstEnum), from(HandleInst))]
        pub enum HandleInstEnum{
            #[byteserde(replace(HandleInst(b'I')))]
            ImbalanceOnly,
            #[byteserde(replace(HandleInst(b'O')))]
            RetailOrderType1,
            #[byteserde(replace(HandleInst(b'T')))]
            RetailOrderType2,
            #[byteserde(replace(HandleInst(b'Q')))]
            RetailPriceImprovement,
            #[byteserde(replace(HandleInst(b'B')))]
            ExtendedLifeContinuous,
            #[byteserde(replace(HandleInst(b'D')))]
            DirectListingCapitalRaise,
            #[byteserde(replace(HandleInst(b'R')))]
            HiddenPriceImprovement,
        }
    }
    pub mod bbo_weight_indicator{
        use super::*;
        char_ascii!(BBOWeightIndicator, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(BBOWeightIndicator, 18);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(BBOWeightIndicator), from(BBOWeightIndicatorEnum), from(BBOWeightIndicator))]
        pub enum BBOWeightIndicatorEnum{
            #[byteserde(replace(BBOWeightIndicator(b'0')))]
            ZeroPoint2,
            #[byteserde(replace(BBOWeightIndicator(b'1')))]
            Point2One,
            #[byteserde(replace(BBOWeightIndicator(b'2')))]
            OneTwo,
            #[byteserde(replace(BBOWeightIndicator(b'3')))]
            TwoAbove,
            #[byteserde(replace(BBOWeightIndicator(b' ')))]
            Unspecified,
            #[byteserde(replace(BBOWeightIndicator(b'S')))]
            SetsQBBOWhileJoiningNBBO,
            #[byteserde(replace(BBOWeightIndicator(b'N')))]
            ImprovesNBBOUponEntry
        }
    }
    pub mod display_qty{
        use super::*;
        u32_tuple!(DisplayQty, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(DisplayQty, 22);
    }
    pub mod display_price{
        use super::*;
        u64_tuple!(DisplayPrice, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(DisplayPrice, 23);
    }
    pub mod group_id{
        use super::*;
        u16_tuple!(GroupId, "be", ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Debug, Clone);
        option_tag!(GroupId, 24);
    }
    pub mod shares_located{
        use super::*;
        char_ascii!(SharesLocated, ByteSerializeStack, ByteDeserialize, ByteSerializedSizeOf, ByteSerializedLenOf, PartialEq, Clone);
        option_tag!(SharesLocated, 25);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(SharesLocated), from(SharesLocatedEnum), from(SharesLocated))]
        pub enum SharesLocatedEnum{
            #[byteserde(replace(SharesLocated(b'Y')))]
            Yes,
            #[byteserde(replace(SharesLocated(b'N')))]
            No,
        }
    }
}

#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, ByteSerializedLenOf, Debug, Clone)]
pub struct TagValueElement<T>
where
    T: ByteSerializeStack + ByteDeserialize<T> + ByteSerializedLenOf,
{
    length: u8,
    option_tag: u8,
    option_value: T,
}
impl<T> TagValueElement<T>
where
    T: ByteSerializeStack + ByteDeserialize<T> + OptionTag + ByteSerializedLenOf,
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

#[derive(ByteSerializeStack, ByteDeserialize, ByteSerializedLenOf, PartialEq, Debug)]
#[byteserde(peek(1, 1))] // peek(start, len) -> peek one byte after keeping one
pub struct OptionalAppendage {
    #[byteserde(eq(SecondaryOrdRefNum::tag_as_slice()))]
    pub secondary_ord_ref_num: Option<TagValueElement<SecondaryOrdRefNum>>,

    #[byteserde(eq(Firm::tag_as_slice()))]
    pub firm: Option<TagValueElement<Firm>>,

    #[byteserde(eq(MinQty::tag_as_slice()))]
    pub min_quantity: Option<TagValueElement<MinQty>>,

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
            min_quantity: None,
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
        min_quantity: Some(msg_min_qty.clone()),
        ..Default::default()
    };

    let mut ser = ByteSerializerStack::<128>::default();
    ser.serialize(&inp_appendage).unwrap();
    info!("ser: {:#x}", ser);

    let mut des = ByteDeserializer::new(ser.as_slice());
    let out_appendage = OptionalAppendage::byte_deserialize(&mut des).unwrap();
    info!("inp_appendage: {:?}", inp_appendage);
    info!("out_appendage: {:?}", out_appendage);
    assert_eq!(inp_appendage, out_appendage);
}
