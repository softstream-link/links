pub mod setup {

    pub mod model {
        pub const TEXT_SIZE: usize = 20;
        use crate::prelude::*;
        use bytes::{Bytes, BytesMut};
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
        use byteserde_types::prelude::*;

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestCltMsgDebug {
            ty: ConstCharAscii<b'1'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl TestCltMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestCltMsgLoginReq {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestSvcMsgLoginAcpt {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestSvcMsgDebug {
            ty: ConstCharAscii<b'2'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl TestSvcMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestHBeatMsgDebug {
            ty: ConstCharAscii<b'H'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl TestHBeatMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
        #[byteserde(peek(0, 1))]
        pub enum TestCltMsg {
            #[byteserde(eq(&[b'1']))]
            Dbg(TestCltMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Login(TestCltMsgLoginReq),
            #[byteserde(eq(&[b'H']))]
            HBeat(TestHBeatMsgDebug),
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, )]
        #[byteserde(peek(0, 1))]
        pub enum TestSvcMsg {
            #[byteserde(eq(&[b'2']))]
            Dbg(TestSvcMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Accept(TestSvcMsgLoginAcpt),
            #[byteserde(eq(&[b'H']))]
            HBeat(TestHBeatMsgDebug),
        }

        #[derive(PartialEq, Clone, Debug)]
        pub enum TestMsg {
            Clt(TestCltMsg),
            Svc(TestSvcMsg),
        }
        impl From<TestCltMsg> for TestMsg {
            fn from(msg: TestCltMsg) -> Self {
                Self::Clt(msg)
            }
        }
        impl From<TestSvcMsg> for TestMsg {
            fn from(msg: TestSvcMsg) -> Self {
                Self::Svc(msg)
            }
        }

        pub struct MsgFramer;
        const FRAME_SIZE: usize = TEXT_SIZE + 1;
        impl Framer for MsgFramer {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() < FRAME_SIZE {
                    return None;
                } else {
                    let frame = bytes.split_to(FRAME_SIZE);
                    return Some(frame.freeze());
                }
            }
        }
        #[cfg(test)]
        mod test {
            use super::*;
            use byteserde::size::ByteSerializedLenOf;
            // for simplicity the framer assume each message to be of fixed size, this test just to avoid mistakes
            #[test]
            fn test_msg_len() {
                assert_eq!(TestCltMsgDebug::default().byte_len(), FRAME_SIZE);
                assert_eq!(TestCltMsgLoginReq::default().byte_len(), FRAME_SIZE);
                assert_eq!(TestSvcMsgDebug::default().byte_len(), FRAME_SIZE);
                assert_eq!(TestSvcMsgLoginAcpt::default().byte_len(), FRAME_SIZE);
                assert_eq!(TestHBeatMsgDebug::default().byte_len(), FRAME_SIZE);
            }
        }
    }
    pub mod protocol {

        use std::{error::Error, time::Duration};

        use bytes::{Bytes, BytesMut};
        use log::info;

        use crate::prelude::*;

        use super::model::*;

        #[derive(Debug, Clone, PartialEq)]
        pub struct TestCltMsgProtocol;
        impl Messenger for TestCltMsgProtocol {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
        }
        impl Framer for TestCltMsgProtocol {
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
                clt: CltSender<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                loop {
                    let mut msg = TestSvcMsg::HBeat(TestHBeatMsgDebug::new(b"svc ping"));
                    clt.send(&mut msg).await?;
                    tokio::time::sleep(Duration::from_secs(1)).await;
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
                clt: CltSender<P, C, MMS>,
            ) -> Result<(), Box<dyn Error+Send+Sync>> {
                loop {
                    let mut msg = TestCltMsg::HBeat(TestHBeatMsgDebug::new(b"clt ping"));
                    clt.send(&mut msg).await?;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}
