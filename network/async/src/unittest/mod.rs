pub mod setup {

    pub mod model {
        pub const TEXT_SIZE: usize = 20;
        use crate::prelude::*;
        use bytes::{Bytes, BytesMut};
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
        use byteserde_types::prelude::*;

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct CltMsgDebug {
            ty: ConstCharAscii<b'1'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl CltMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct CltMsgLoginReq {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct SvcMsgLoginAcpt {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct SvcMsgDebug {
            ty: ConstCharAscii<b'2'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl SvcMsgDebug {
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
        pub enum CltMsg {
            #[byteserde(eq(&[b'1']))]
            Dbg(CltMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Login(CltMsgLoginReq),
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, )]
        #[byteserde(peek(0, 1))]
        pub enum SvcMsg {
            #[byteserde(eq(&[b'2']))]
            Dbg(SvcMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Accept(SvcMsgLoginAcpt),
        }

        #[derive(PartialEq, Clone, Debug)]
        pub enum Msg {
            Clt(CltMsg),
            Svc(SvcMsg),
        }
        impl From<CltMsg> for Msg {
            fn from(msg: CltMsg) -> Self {
                Self::Clt(msg)
            }
        }
        impl From<SvcMsg> for Msg {
            fn from(msg: SvcMsg) -> Self {
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
                assert_eq!(CltMsgDebug::default().byte_len(), FRAME_SIZE);
                assert_eq!(CltMsgLoginReq::default().byte_len(), FRAME_SIZE);
                assert_eq!(SvcMsgDebug::default().byte_len(), FRAME_SIZE);
                assert_eq!(SvcMsgLoginAcpt::default().byte_len(), FRAME_SIZE);
            }
        }
    }
    pub mod protocol {

        use std::error::Error;

        use bytes::{Bytes, BytesMut};
        use log::info;

        use crate::prelude::*;

        use super::model::*;

        #[derive(Debug, Clone, PartialEq)]
        pub struct CltMsgProtocol;

        impl Messenger for CltMsgProtocol {
            type SendMsg = CltMsg;
            type RecvMsg = SvcMsg;
        }
        impl Framer for CltMsgProtocol {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct SvcMsgProtocol;
        impl Protocol for CltMsgProtocol {
            async fn init_sequence<
                P: Protocol<SendMsg = Self::SendMsg, RecvMsg = Self::RecvMsg>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &self,
                clt: &Clt<P, C, MMS>,
            ) -> Result<(), Box<dyn Error + Send + Sync>> {
                let login = CltMsg::Login(CltMsgLoginReq::default());
                clt.send(&login).await?;

                info!("{}->{:?}", clt.con_id(), login);
                let msg = clt.recv().await?;

                match msg {
                    Some(SvcMsg::Accept(acpt)) => {
                        info!("{}<-{:?}", clt.con_id(), acpt);
                        Ok(())
                    }
                    _ => Err(format!("Not Expected {}<-{:?}", clt.con_id(), msg).into()),
                }
            }
        }
        impl Protocol for SvcMsgProtocol {
            async fn init_sequence<
                P: Protocol<SendMsg = Self::SendMsg, RecvMsg = Self::RecvMsg>,
                C: CallbackSendRecv<P>,
                const MMS: usize,
            >(
                &self,
                clt: &Clt<P, C, MMS>,
            ) -> Result<(), Box<dyn Error + Send + Sync>> {
                let login = clt.recv().await?;
                info!("{}<-{:?}", clt.con_id(), login);
                let auth = SvcMsg::Accept(SvcMsgLoginAcpt::default());
                clt.send(&auth).await?;
                info!("{}->{:?}", clt.con_id(), auth);
                Ok(())
            }
        }
        impl Messenger for SvcMsgProtocol {
            type SendMsg = SvcMsg;
            type RecvMsg = CltMsg;
        }
        impl Framer for SvcMsgProtocol {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                MsgFramer::get_frame(bytes)
            }
        }
    }
}
