use byteserde::prelude::*;
use byteserde_derive::{ByteDeserialize, ByteSerializeStack};
use byteserde_types::{char_ascii, string_ascii_fixed, u32_tuple, u64_tuple};
use log::info;
pub use option_values::*;

use crate::unittest::setup;

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
pub mod option_values{
    use super::*;
    u64_tuple!(SecondaryOrdRefNum, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
    option_tag!(SecondaryOrdRefNum, 1);
    
    string_ascii_fixed!(Firm, 4, b' ', true, ByteSerializeStack, ByteDeserialize, PartialEq);
    option_tag!(Firm, 2);
    
    u32_tuple!(MinQty, "be", ByteSerializeStack, ByteDeserialize, PartialEq, Debug);
    option_tag!(MinQty, 3);
    
    char_ascii!(CustomerType, ByteSerializeStack, ByteDeserialize, PartialEq);
    option_tag!(CustomerType, 4);
    
    u32_tuple!(MaxFloor, "be", ByteSerializeStack, ByteDeserialize, PartialEq);
    option_tag!(MaxFloor, 5);
    
    char_ascii!(PriceType, ByteSerializeStack, ByteDeserialize, PartialEq);
    option_tag!(PriceType, 6);
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
