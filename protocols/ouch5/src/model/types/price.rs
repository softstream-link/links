use byteserde_derive::{ByteSerializeStack, ByteDeserialize};




#[derive(ByteSerializeStack, ByteDeserialize, PartialEq, Debug)]
#[byteserde(endian = "be")]
pub struct Price(u64);
impl From<f64> for Price {
    fn from(f: f64) -> Self {
        Price((f * 10000.0) as u64)
    }
}