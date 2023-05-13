
// http://www.nasdaqtrader.com/content/technicalsupport/specifications/dataproducts/soupbintcp.pdf
pub struct SoupBinTcpV3 {
    pub packet_length: i16, // a two byte big-endian length that indicates the length of rest of the packe (meaning the length of the payload plus the length of the packet type – which is 1)
    pub packet_type: char, // a single byte header which indicates the packet type
}

pub struct LoginAccepted {
    pub soup: SoupBinTcpV3,
    pub session: String,
}

// struct EnterOderMessage {
//     header: SoupBin,
//     message_type: char,
//     order_token: u
// }