pub mod setup {

    pub mod framer {

        use bytes::{Bytes, BytesMut};

        use crate::prelude::*;

        use links_testing::unittest::setup::model::*;

        pub struct TestMsgFramer;
        impl Framer for TestMsgFramer {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() < TEST_MSG_FRAME_SIZE {
                    return None;
                } else {
                    let frame = bytes.split_to(TEST_MSG_FRAME_SIZE);
                    return Some(frame.freeze());
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
        pub use super::framer::TestCltMsgProtocol;
        pub use super::framer::TestSvcMsgProtocol;

        use crate::prelude::*;
        use links_testing::unittest::setup::model::*;
        impl MessengerOld for TestSvcMsgProtocol {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;
        }
        impl MessengerOld for TestCltMsgProtocol {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
        }
    }
}
