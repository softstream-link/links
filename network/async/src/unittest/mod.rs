pub mod setup {

    pub mod model {
        pub const TEXT_SIZE: usize = 20;
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
        use byteserde_types::prelude::*;

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,)]
        pub struct CltDebugMsg {
            ty: ConstCharAscii<b'1'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl CltDebugMsg {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,)]
        pub struct CltLoginRequest {
            pub ty: ConstCharAscii<b'L'>,
        }
        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,)]
        pub struct SvcLoginAccept {
            pub ty: ConstCharAscii<b'A'>,
        }

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,)]
        pub struct SvcDebugMsg {
            ty: ConstCharAscii<b'2'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl SvcDebugMsg {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[derive(
            ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
        )]
        #[byteserde(peek(0, 1))]
        pub enum CltMsg {
            #[byteserde(eq(&[b'1']))]
            Dbg(CltDebugMsg),
            #[byteserde(eq(&[b'L']))]
            Login(CltLoginRequest),
        }

        #[derive(
            ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
        )]
        #[byteserde(peek(0, 1))]
        pub enum SvcMsg {
            #[byteserde(eq(&[b'2']))]
            Dbg(SvcDebugMsg),
            #[byteserde(eq(&[b'A']))]
            Accept(SvcLoginAccept),
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
    }
    pub mod protocol {

        use bytes::{Bytes, BytesMut};

        use crate::prelude::*;

        use super::model::*;

        #[derive(Debug, Clone, PartialEq)]
        pub struct CltMsgProtocol;
        impl Protocol for CltMsgProtocol {
            // async fn init_sequence<PROTOCOL, CALLBACK, const MAX_MSG_SIZE: usize>(
            async fn init_sequence<
                PROTOCOL: Protocol<SendMsg = Self::SendMsg, RecvMsg = Self::RecvMsg>,
                CALLBACK: CallbackSendRecv<PROTOCOL>,
                const MAX_MSG_SIZE: usize,
            >(
                &self,
                clt: &Clt<PROTOCOL, CALLBACK, MAX_MSG_SIZE>,
            ) {
                let msg = CltMsg::Login(CltLoginRequest {
                    ty: Default::default(),
                });
                clt.send(&msg).await.expect(format!("send msg failed {:?}", msg).as_str());
                use log::warn;
                warn!("init_sequence clt {:?}", msg);
                let auth = clt.recv().await.expect("recv auth failed");
                warn!("init_sequence clt {:?}", auth);


                

                ()
            }
        }
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
        impl Protocol for SvcMsgProtocol {
            // async fn init_sequence(&self){}
            async fn init_sequence<
                PROTOCOL: Protocol<SendMsg = Self::SendMsg, RecvMsg = Self::RecvMsg>,
                CALLBACK: CallbackSendRecv<PROTOCOL>,
                const MAX_MSG_SIZE: usize,
            >(
                &self,
                clt: &Clt<PROTOCOL, CALLBACK, MAX_MSG_SIZE>,
            ) {
                use log::warn;
                warn!("init_sequence svc in impl");
                let auth = clt.recv().await.expect("recv auth failed");
                warn!("init_sequence svc {:?}", auth);
                let msg = SvcMsg::Accept(SvcLoginAccept {
                    ty: Default::default(),
                });
                clt.send(&msg).await.expect(format!("send msg failed {:?}", msg).as_str());
                warn!("init_sequence svc {:?}", msg);
                ()
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

        pub struct MsgFramer;
        impl Framer for MsgFramer {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                let msg_size: usize = TEXT_SIZE + 1;
                if bytes.len() < msg_size {
                    return None;
                } else {
                    let frame = bytes.split_to(msg_size);
                    return Some(frame.freeze());
                }
            }
        }
    }
}
