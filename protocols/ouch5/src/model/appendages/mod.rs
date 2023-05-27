use byteserde::prelude::*;
use byteserde_derive::{ByteDeserialize, ByteEnumFromBinder, ByteSerializeStack};
use byteserde_types::{
    char_ascii, i32_tuple, numeric_tuple, string_ascii_fixed, u16_tuple, u32_tuple, u64_tuple,
};

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
    discression_peg_offset::*,
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
}
macro_rules! option_tag {
    ($name:ident, $tag:literal) => {
        impl OptionTag for $name {
            fn tag() -> u8 {
                $tag
            }
        }
    };
}

#[rustfmt::skip]
mod optional_value{
    use super::*;
    pub mod secondary_ord_ref_num{
        use super::*;
        u64_tuple!(SecondaryOrdRefNum, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(SecondaryOrdRefNum, 1);
    }
    pub mod firm {
        use super::*;
        string_ascii_fixed!(Firm, 4, b' ', true, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(Firm, 2);
    }
    pub mod min_qty {
        use super::*;
        u32_tuple!(MinQty, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(MinQty, 3);
    }
    pub mod customer_type{
        use super::*;
        char_ascii!(CustomerType, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(CustomerType, 4);
    }
    pub mod max_floor{
        use super::*;
        u32_tuple!(MaxFloor, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(MaxFloor, 5);
    }
    pub mod price_type{
        use super::*;
        char_ascii!(PriceType, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(PriceType, 6);
        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(PriceType))]
        #[byteserde(from(PriceType))]
        #[byteserde(from(PriceTypeEnum))]
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
        i32_tuple!(PegOffset, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(PegOffset, 7);
    } 
    pub mod discretion_price{
        use super::*;
        u64_tuple!(DiscretionPrice, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(DiscretionPrice, 9);
    }
    pub mod discretion_price_type{
        use super::*;
        char_ascii!(DiscretionPriceType, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(DiscretionPriceType, 10);
        
        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(DiscretionPriceType))]
        #[byteserde(from(DiscretionPriceTypeEnum))]
        #[byteserde(from(DiscretionPriceType))]
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
    pub mod discression_peg_offset{
        use super::*;
        i32_tuple!(DiscressionPegOffset, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(DiscressionPegOffset, 11);
    }
    pub mod post_only{
        use super::*;
        char_ascii!(PostOnly, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(PostOnly, 12);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(PostOnly))]
        #[byteserde(from(PostOnlyEnum))]
        #[byteserde(from(PostOnly))]
        pub enum PostOnlyEnum{
            #[byteserde(replace(PostOnly(b'P')))]
            PostOnly,
            #[byteserde(replace(PostOnly(b'N')))]
            No,
        }
    }
    pub mod random_reserves{
        use super::*;
        u32_tuple!(RandomReserves, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(RandomReserves, 13);
    }
    pub mod route{
        use super::*;
        string_ascii_fixed!(Route, 4, b' ', true, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(Route, 14);
    }
    pub mod expire_time{
        use super::*;
        u32_tuple!(ExpireTime, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(ExpireTime, 15);
    }
    pub mod trade_now{
        use super::*;
        char_ascii!(TradeNow, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(TradeNow, 16);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(TradeNow))]
        #[byteserde(from(TradeNowEnum))]
        #[byteserde(from(TradeNow))]
        pub enum TradeNowEnum{
            #[byteserde(replace(TradeNow(b'Y')))]
            Yes,
            #[byteserde(replace(TradeNow(b'N')))]
            No,
        }
    }
    pub mod handle_inst{
        use super::*;
        char_ascii!(HandleInst, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(HandleInst, 17);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(HandleInst))]
        #[byteserde(from(HandleInstEnum))]
        #[byteserde(from(HandleInst))]
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
        char_ascii!(BBOWeightIndicator, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(BBOWeightIndicator, 18);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(BBOWeightIndicator))]
        #[byteserde(from(BBOWeightIndicatorEnum))]
        #[byteserde(from(BBOWeightIndicator))]
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
        u32_tuple!(DisplayQty, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(DisplayQty, 22);
    }
    pub mod display_price{
        use super::*;
        u64_tuple!(DisplayPrice, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(DisplayPrice, 23);
    }
    pub mod group_id{
        use super::*;
        u16_tuple!(GroupId, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
        option_tag!(GroupId, 24);
    }
    pub mod shares_located{
        use super::*;
        char_ascii!(SharesLocated, ByteSerializeStack, ByteDeserialize, PartialEq);
        option_tag!(SharesLocated, 25);

        #[derive(ByteEnumFromBinder)]
        #[byteserde(bind(SharesLocated))]
        #[byteserde(from(SharesLocatedEnum))]
        #[byteserde(from(SharesLocated))]
        pub enum SharesLocatedEnum{
            #[byteserde(replace(SharesLocated(b'Y')))]
            Yes,
            #[byteserde(replace(SharesLocated(b'N')))]
            No,
        }
    }
}

#[derive(ByteSerializeStack, ByteDeserialize, Debug)]
pub struct TagValueElement<T>
where
    T: ByteSerializeStack + ByteDeserialize<T>,
{
    length: u8,
    option_tag: u8,
    option_value: T,
}
impl<T> TagValueElement<T>
where
    T: ByteSerializeStack + ByteDeserialize<T> + OptionTag,
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

pub struct Appendage {
    secondary_ord_ref_num: Option<TagValueElement<SecondaryOrdRefNum>>,
    firm: Option<TagValueElement<Firm>>,
    min_quantity: Option<TagValueElement<MinQty>>,
    customer_type: Option<TagValueElement<CustomerType>>,
    max_floor: Option<TagValueElement<MaxFloor>>,
    price_type: Option<TagValueElement<PriceType>>,
}
#[test]
fn tag_value_elements() {
    use crate::unittest::setup;
    use log::info;
    setup::log::configure();
    // use super::option_values::*;
    let msg_sec_ord_ref = TagValueElement::<SecondaryOrdRefNum>::new(SecondaryOrdRefNum::new(1));
    let msg_firm = TagValueElement::<Firm>::new(Firm::new(*b"ABCD"));
    let msg_min_qty = TagValueElement::<MinQty>::new(MinQty::new(1));
    info!("msg_sec_ord_ref: {:?}", msg_sec_ord_ref);
    info!("msg_firm: {:?}", msg_firm);
    info!("msg_min_qty: {:?}", msg_min_qty);

    let mut ser = ByteSerializerStack::<128>::default();
    ser.serialize(&msg_sec_ord_ref).unwrap();
    info!("ser: {:#x}", ser);
    ser.serialize(&msg_firm).unwrap();
    info!("ser: {:#x}", ser);
    ser.serialize(&msg_min_qty).unwrap();
    info!("ser: {:#x}", ser);

    let mut des = ByteDeserializer::new(ser.as_slice());

    let msg_sec_ord_ref: TagValueElement<SecondaryOrdRefNum> = des.deserialize().unwrap();
    let msg_firm: TagValueElement<Firm> = des.deserialize().unwrap();
    let msg_min_qty: TagValueElement<MinQty> = des.deserialize().unwrap();
    info!("msg_sec_ord_ref: {:?}", msg_sec_ord_ref);
    info!("msg_firm: {:?}", msg_firm);
    info!("msg_min_qty: {:?}", msg_min_qty);
}
