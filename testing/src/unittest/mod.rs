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
        use std::{net::TcpListener, time::Duration};

        use lazy_static::lazy_static;
        lazy_static! {
            static ref AVAILABLE_PORT: u16 = {
                for port in 8000..9000 {
                    match TcpListener::bind(format!("0.0.0.0:{}", port)) {
                        Ok(_) => return port,
                        _ => {}
                    }
                }
                panic!("Unable to find an available port in range 8000..9000");
            };
        }

        pub fn default_addr() -> String {
            String::from(format!("0.0.0.0:{}", *AVAILABLE_PORT))
        }

        pub fn default_connect_timeout() -> Duration {
            Duration::from_secs_f32(0.5)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn default_find_timeout() -> Option<Duration> {
            Some(Duration::from_secs_f32(1.))
        }
    }

    // pub mod model {
    //     pub const TEXT_SIZE: usize = 20;
    //     use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
    //     use byteserde_types::prelude::*;

    //     #[derive(
    //         ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
    //     )]
    //     pub struct CltMsg {
    //         ty: ConstCharAscii<b'1'>,
    //         text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
    //     }
    //     impl CltMsg {
    //         pub fn new(text: &[u8]) -> Self {
    //             Self {
    //                 ty: Default::default(),
    //                 text: StringAsciiFixed::from(text),
    //             }
    //         }
    //     }
    //     #[derive(
    //         ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug,
    //     )]
    //     pub struct SvcMsg {
    //         ty: ConstCharAscii<b'2'>,
    //         text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
    //     }
    //     impl SvcMsg {
    //         pub fn new(text: &[u8]) -> Self {
    //             Self {
    //                 ty: Default::default(),
    //                 text: StringAsciiFixed::from(text),
    //             }
    //         }
    //     }

    //     #[derive(PartialEq, Clone, Debug)]
    //     pub enum Msg {
    //         Clt(CltMsg),
    //         Svc(SvcMsg),
    //     }
    // }
    // pub mod protocol {
    //     use bytes::{Bytes, BytesMut};

    //     use crate::prelude::*;

    //     use super::model::*;

    //     #[derive(Debug, Clone, PartialEq)]
    //     pub struct CltMsgProtocol;
    //     impl Protocol for CltMsgProtocol {}
    //     impl Messenger for CltMsgProtocol {
    //         type SendMsg = CltMsg;
    //         type RecvMsg = SvcMsg;
    //     }
    //     impl Framer for CltMsgProtocol{
    //         fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
    //             MsgFramer::get_frame(bytes)
    //         }
    //     }

    //     #[derive(Debug, Clone, PartialEq)]
    //     pub struct SvcMsgProtocol;
    //     impl Protocol for SvcMsgProtocol {}
    //     impl Messenger for SvcMsgProtocol {
    //         type SendMsg = SvcMsg;
    //         type RecvMsg = CltMsg;
    //     }
    //     impl Framer for SvcMsgProtocol{
    //         fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
    //             MsgFramer::get_frame(bytes)
    //         }
    //     }

    //     pub struct MsgFramer;
    //     impl Framer for MsgFramer {
    //         fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
    //             let msg_size: usize = TEXT_SIZE + 1;
    //             if bytes.len() < msg_size {
    //                 return None;
    //             } else {
    //                 let frame = bytes.split_to(msg_size);
    //                 return Some(frame.freeze());
    //             }
    //         }
    //     }
    // }
}
