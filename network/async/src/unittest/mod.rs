pub mod setup {

    pub mod protocol {
        use links_network_core::prelude::{CallbackSendRecv, Framer, Messenger};
        use links_testing::unittest::setup::model::*;

        use std::{error::Error, time::Duration};

        use bytes::{Bytes, BytesMut};
        use log::info;

        use crate::prelude::*;
        pub struct MsgFramer;
        impl Framer for MsgFramer {
            const MAX_FRAME_SIZE: usize = TEST_MSG_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() <  Self::MAX_FRAME_SIZE {
                    return None;
                } else {
                    let frame = bytes.split_to(Self::MAX_FRAME_SIZE);
                    return Some(frame.freeze());
                }
            }
        }

        // use super::model::*;
        pub const HBEAT_INTERVAL: Duration = Duration::from_millis(500);
        #[derive(Debug, Clone, PartialEq)]
        pub struct TestCltMsgProtocol;
        impl Messenger for TestCltMsgProtocol {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
        }
        impl Framer for TestCltMsgProtocol {
            const MAX_FRAME_SIZE: usize = MsgFramer::MAX_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct TestSvcMsgProtocol;
        impl Messenger for TestSvcMsgProtocol {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;
        }
        impl Framer for TestSvcMsgProtocol {
            const MAX_FRAME_SIZE: usize = MsgFramer::MAX_FRAME_SIZE;
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }

        impl Protocol for TestSvcMsgProtocol {
            async fn handshake<
                's,
                P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &'s self,
                clt: &'s Clt<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                let login = clt.recv().await?;
                info!("{}<-{:?}", clt.con_id(), login);
                let mut auth = TestSvcMsg::Accept(TestSvcMsgLoginAcpt::default());
                clt.send(&mut auth).await?;
                info!("{}->{:?}", clt.con_id(), auth);
                Ok(())
            }
            async fn keep_alive_loop<
                P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &self,
                clt: CltSenderAsync<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                loop {
                    let mut msg = TestSvcMsg::HBeat(TestHBeatMsgDebug::new(b"svc ping"));
                    clt.send(&mut msg).await?;
                    tokio::time::sleep(HBEAT_INTERVAL).await;
                }
            }
        }

        impl Protocol for TestCltMsgProtocol {
            async fn handshake<
                's,
                P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &'s self,
                clt: &'s Clt<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                let mut login = TestCltMsg::Login(TestCltMsgLoginReq::default());
                clt.send(&mut login).await?;

                info!("{}->{:?}", clt.con_id(), login);
                let msg = clt.recv().await?;

                match msg {
                    Some(TestSvcMsg::Accept(acpt)) => {
                        info!("{}<-{:?}", clt.con_id(), acpt);
                        Ok(())
                    }
                    _ => Err(format!("Not Expected {}<-{:?}", clt.con_id(), msg).into()),
                }
            }
            async fn keep_alive_loop<
                P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &self,
                clt: CltSenderAsync<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                loop {
                    let mut msg = TestCltMsg::HBeat(TestHBeatMsgDebug::new(b"clt ping"));
                    clt.send(&mut msg).await?;
                    tokio::time::sleep(HBEAT_INTERVAL).await;
                }
            }
        }
    }
}
