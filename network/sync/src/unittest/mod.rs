pub mod setup {

    pub mod framer {

        use bytes::{Bytes, BytesMut};
        use links_network_core::prelude::*;
        pub use links_testing::unittest::setup::model::TEST_MSG_FRAME_SIZE;
        pub struct TestMsgFramer;
        impl Framer for TestMsgFramer {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() < TEST_MSG_FRAME_SIZE {
                    None
                } else {
                    let frame = bytes.split_to(TEST_MSG_FRAME_SIZE);
                    Some(frame.freeze())
                }
            }
        }
        #[derive(Debug, Clone, PartialEq)]
        pub struct TestCltMsgProtocol;

        impl Framer for TestCltMsgProtocol {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                TestMsgFramer::get_frame(bytes)
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct TestSvcMsgProtocol;

        impl Framer for TestSvcMsgProtocol {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                TestMsgFramer::get_frame(bytes)
            }
        }
    }
    pub mod messenger {
        use std::io::Error;

        pub use super::framer::TestCltMsgProtocol;
        pub use super::framer::TestSvcMsgProtocol;
        use byteserde::prelude::{from_slice, to_bytes_stack};

        use links_network_core::prelude::*;
        use links_testing::unittest::setup::model::*;

        impl MessengerNew for TestSvcMsgProtocol {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;

            #[inline(always)]
            fn serialize<const MMS: usize>(msg: &Self::SendT) -> Result<([u8; MMS], usize), Error> {
                match to_bytes_stack::<MMS, Self::SendT>(msg) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
                }
            }

            #[inline(always)]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error> {
                match from_slice::<Self::RecvT>(frame) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
                }
            }
        }
        impl MessengerNew for TestCltMsgProtocol {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
            #[inline(always)]
            fn serialize<const MMS: usize>(msg: &Self::SendT) -> Result<([u8; MMS], usize), Error> {
                match to_bytes_stack::<MMS, Self::SendT>(msg) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
                }
            }

            #[inline(always)]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error> {
                match from_slice::<Self::RecvT>(frame) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e)),
                }
            }
        }
    }
}
