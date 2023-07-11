use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use framing::{prelude::*, MessageHandler};

use crate::prelude::SoupBin;

// use crate::model::clt_heartbeat::ClientHeartbeat;
// pub struct SoupBinTcp4FrameHandler;

#[derive(Debug)]
pub struct SoupBinFrame;

impl FrameHandler for SoupBinFrame {
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

#[derive(Debug)]
pub struct SoupBinMessageHandler<PAYLOAD> {
    phantom: std::marker::PhantomData<PAYLOAD>,
}

#[rustfmt::skip]
impl<PAYLOAD, const STACK_SIZE: usize> MessageHandler<STACK_SIZE> for SoupBinMessageHandler<PAYLOAD>
where
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + std::fmt::Debug + Clone,
{
    type MSG = SoupBin<PAYLOAD>;
    type FHANDLER = SoupBinFrame;
}


#[cfg(test)]
mod test {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use log::info;

    use crate::{
        model::{payload::SamplePayload, soup_bin::SoupBin, unsequenced_data::UnsequencedData},
        prelude::*,
        unittest::setup,
    };

    #[test]
    fn test_soup_bin_admin() {
        setup::log::configure();
        const CAP: usize = 1024;
        let mut ser = ByteSerializerStack::<CAP>::default();
        let msg_inp = vec![
            SoupBin::CltHBeat(CltHeartbeat::default()),
            SoupBin::SvcHBeat(SvcHeartbeat::default()),
            SoupBin::Dbg(Debug::default()),
            SoupBin::End(EndOfSession::default()),
            SoupBin::LoginReq(LoginRequest::default()),
            SoupBin::LoginAcc(LoginAccepted::default()),
            SoupBin::LoginRej(LoginRejected::not_authorized()),
            SoupBin::LogoutReq(LogoutRequest::default()),
            SoupBin::SData(SequencedData::<SamplePayload>::default()),
            SoupBin::UData(UnsequencedData::<SamplePayload>::default()),
        ];
        for msg in msg_inp.iter() {
            info!("msg_inp {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut bytes = BytesMut::with_capacity(CAP);
        bytes.put_slice(ser.as_slice());

        let mut msg_out: Vec<SoupBin<SamplePayload>> = vec![];
        loop {
            let frame = SoupBinFrame::get_frame(&mut bytes);
            match frame {
                Some(frame) => {
                    let des = &mut ByteDeserializerSlice::new(&frame[..]);
                    let msg = SoupBin::byte_deserialize(des).unwrap();
                    info!("msg_out {:?}", msg);
                    msg_out.push(msg);
                }
                None => break,
            }
        }
        assert_eq!(msg_inp, msg_out);
    }
}
