pub mod setup {

    pub mod framer {

        use std::time::Duration;

        use bytes::{Bytes, BytesMut};

        use links_network_core::prelude::*;

        use links_testing::unittest::setup::model::*;
        pub const HBEAT_INTERVAL: Duration = Duration::from_millis(500);

        
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

        use links_network_core::prelude::*;
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
