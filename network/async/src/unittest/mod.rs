pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            SETUP.call_once(|| {
                let _ = env_logger::builder()
                    .format_timestamp_micros()
                    // .is_test(true) // disables color in the terminal
                    .filter_level(log::LevelFilter::Trace)
                    .try_init();
            });
        }
    }
    pub mod net {
        use std::time::Duration;

        pub fn default_addr() -> String {
            String::from("0.0.0.0:8080")
        }
        pub fn default_connect_timeout() -> Duration {
            Duration::from_secs_f32(0.5)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn default_find_timeout() -> Duration {
            Duration::from_secs_f32(1.)
        }
    }

    pub mod model {
        pub const TEXT_SIZE: usize = 20;
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
        use byteserde_types::prelude::*;

        #[derive(
            ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
        )]
        pub struct MsgFromClt {
            ty: ConstCharAscii<b'1'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl MsgFromClt {
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
        pub struct MsgFromSvc {
            ty: ConstCharAscii<b'2'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl MsgFromSvc {
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
        pub enum Msg {
            #[byteserde(eq([b'1'].as_slice()))]
            Clt(MsgFromClt),
            #[byteserde(eq([b'2'].as_slice()))]
            Svc(MsgFromSvc),
        }
    }
    pub mod protocol {
        use bytes::{Bytes, BytesMut};

        use crate::prelude::*;

        use super::model::*;

        #[derive(Debug, Clone, PartialEq)]
        pub struct MsgProtocolHandler;
        impl ProtocolHandler for MsgProtocolHandler {}
        impl Messenger for MsgProtocolHandler {
            type Message = Msg;
        }
        impl Framer for MsgProtocolHandler {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                let msg_size = TEXT_SIZE + 1;
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
