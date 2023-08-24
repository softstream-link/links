pub mod setup {

    pub mod framer {

        use bytes::{Bytes, BytesMut};

        use crate::prelude::*;

        use links_testing::unittest::setup::model::*;

        pub struct MsgFramer;
        impl Framer for MsgFramer {
            const MAX_FRAME_SIZE: usize = TEST_MSG_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() < Self::MAX_FRAME_SIZE {
                    return None;
                } else {
                    let frame = bytes.split_to(Self::MAX_FRAME_SIZE);
                    return Some(frame.freeze());
                }
            }
        }
        #[derive(Debug, Clone, PartialEq)]
        pub struct TestCltMsgProtocol;

        impl Framer for TestCltMsgProtocol {
            const MAX_FRAME_SIZE: usize = MsgFramer::MAX_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct TestSvcMsgProtocol;

        impl Framer for TestSvcMsgProtocol {
            const MAX_FRAME_SIZE: usize = MsgFramer::MAX_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }
    }
    pub mod messenger {
        pub use super::framer::TestCltMsgProtocol;
        pub use super::framer::TestSvcMsgProtocol;

        use crate::prelude::*;
        use links_testing::unittest::setup::model::*;
        impl Messenger for TestSvcMsgProtocol {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;
        }
        impl Messenger for TestCltMsgProtocol {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
        }
    }
}
