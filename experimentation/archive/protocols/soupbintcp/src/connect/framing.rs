use links_async::prelude::*;

pub type SoupBinFramer = PacketLengthU16Framer<0, true, true>;

#[cfg(test)]
mod test {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use byteserde::prelude::*;
    use log::info;

    use crate::{
        model::{payload::SamplePayload, soup_bin::SBCltMsg},
        prelude::*,
        unittest::setup::model::{clt_msgs_default, svc_msgs_default},
    };
    use links_core::unittest::setup;

    #[test]
    fn test_soup_bin_clt_framing() {
        setup::log::configure();
        const CAP: usize = 1024;
        let mut ser = ByteSerializerStack::<CAP>::default();
        let msg_inp = clt_msgs_default();
        for msg in msg_inp.iter() {
            info!("msg_inp {:?}", msg);
            let _ = ser.serialize(msg).unwrap();
        }
        info!("ser: {:#x}", ser);

        let mut bytes = BytesMut::with_capacity(CAP);
        bytes.put_slice(ser.as_slice());

        let mut msg_out: Vec<SBCltMsg<SamplePayload>> = vec![];
        loop {
            let frame = SoupBinFramer::get_frame(&mut bytes);
            match frame {
                Some(frame) => {
                    let des = &mut ByteDeserializerSlice::new(&frame[..]);
                    let msg = SBCltMsg::byte_deserialize(des).unwrap();
                    info!("msg_out {:?}", msg);
                    msg_out.push(msg);
                }
                None => break,
            }
        }
        assert_eq!(msg_inp, msg_out);
    }
    #[test]
    fn test_soup_bin_svc_framing() {
        setup::log::configure();
        const CAP: usize = 1024;
        let mut ser = ByteSerializerStack::<CAP>::default();
        let msg_inp = svc_msgs_default();
        for msg in msg_inp.iter() {
            // info!("msg_inp {:?}", msg);
            let len = ser.len();
            let _ = ser.serialize(msg).unwrap();
            info!("msg.len() {}, \tser.len(): {},\tmsg_inp {:?}", ser.len() - len, ser.len(), msg);
        }
        info!("ser: {:#x}", ser);

        let mut bytes = BytesMut::with_capacity(CAP);
        bytes.put_slice(ser.as_slice());

        let mut msg_out: Vec<SBSvcMsg<SamplePayload>> = vec![];
        loop {
            let len = bytes.len();
            // let des = &mut ByteDeserializerSlice::new(&bytes[..]);
            // info!("des: {des:#x}");
            let frame = SoupBinFramer::get_frame(&mut bytes);

            match frame {
                Some(frame) => {
                    let des = &mut ByteDeserializerSlice::new(&frame[..]);
                    let msg = SBSvcMsg::byte_deserialize(des).unwrap();
                    info!("frame.len(): {}, \tbyte.len(): {}, msg_out {:?}", frame.len(), len, msg);
                    msg_out.push(msg);
                }
                None => {
                    info!("frame: None, \t byte.len(): {}", len);
                    break;
                }
            }
        }
        assert_eq!(msg_inp, msg_out);
    }
}
