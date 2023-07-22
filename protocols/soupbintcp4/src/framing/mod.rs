use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
// use framing::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::SoupBinMsg;

#[derive(Debug)]
pub struct SoupBinFramer;

impl Framer for SoupBinFramer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        // ensures there is at least 2 bytes to represet packet_length
        if bytes.len() < 2 {
            return None;
        }

        // access packet length with out advancing the cursor, below is a take of the bytes::Buf::get_u16() method
        let packet_length = {
            const SIZE: usize = std::mem::size_of::<u16>();
            // try to convert directly from the bytes
            // this Option<ret> trick is to avoid keeping a borrow on self
            // when advance() is called (mut borrow) and to call bytes() only once
            let ret = bytes
                // .chunk()
                .get(..SIZE)
                .map(|src| unsafe { u16::from_be_bytes(*(src as *const _ as *const [_; SIZE])) });

            if let Some(ret) = ret {
                ret
            } else {
                // if not we copy the bytes in a temp buffer then convert
                let mut buf = [0_u8; SIZE];
                let packet_length = &bytes[..SIZE];
                buf[0] = packet_length[0];
                buf[1] = packet_length[1];
                u16::from_be_bytes(buf)
            }
        };

        // ensure that there is a full frame available in the buffer
        let frame_length = (packet_length + 2) as usize;
        if bytes.len() < frame_length {
            return None;
        } else {
            let frame = bytes.split_to(frame_length);
            return Some(frame.freeze());
        }
    }
}

#[rustfmt::skip]
#[derive(Debug, Clone)]
pub struct SoupBinProtocolHandler<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{ 
    phantom: std::marker::PhantomData<PAYLOAD> 
}
#[rustfmt::skip]
impl<PAYLOAD> Messenger for SoupBinProtocolHandler<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    type Message = SoupBinMsg<PAYLOAD>;
}

#[rustfmt::skip]
impl<PAYLOAD> Framer for SoupBinProtocolHandler<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}
#[rustfmt::skip]
impl<PAYLOAD> ProtocolHandler for SoupBinProtocolHandler<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use log::info;

    use crate::{
        model::{payload::SamplePayload, soup_bin::SoupBinMsg, unsequenced_data::UnsequencedData},
        prelude::*,
        unittest::setup,
    };

    #[test]
    fn test_soup_bin_admin() {
        setup::log::configure();
        const CAP: usize = 1024;
        let mut ser = ByteSerializerStack::<CAP>::default();
        let msg_inp = vec![
            SoupBinMsg::CltHBeat(CltHeartbeat::default()),
            SoupBinMsg::SvcHBeat(SvcHeartbeat::default()),
            SoupBinMsg::Dbg(crate::prelude::Debug::default()),
            SoupBinMsg::End(EndOfSession::default()),
            SoupBinMsg::LoginReq(LoginRequest::default()),
            SoupBinMsg::LoginAcc(LoginAccepted::default()),
            SoupBinMsg::LoginRej(LoginRejected::not_authorized()),
            SoupBinMsg::LogoutReq(LogoutRequest::default()),
            SoupBinMsg::SData(SequencedData::<SamplePayload>::default()),
            SoupBinMsg::UData(UnsequencedData::<SamplePayload>::default()),
        ];
        for msg in msg_inp.iter() {
            info!("msg_inp {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut bytes = BytesMut::with_capacity(CAP);
        bytes.put_slice(ser.as_slice());

        let mut msg_out: Vec<SoupBinMsg<SamplePayload>> = vec![];
        loop {
            let frame = SoupBinFramer::get_frame(&mut bytes);
            match frame {
                Some(frame) => {
                    let des = &mut ByteDeserializerSlice::new(&frame[..]);
                    let msg = SoupBinMsg::byte_deserialize(des).unwrap();
                    info!("msg_out {:?}", msg);
                    msg_out.push(msg);
                }
                None => break,
            }
        }
        assert_eq!(msg_inp, msg_out);
    }
}
