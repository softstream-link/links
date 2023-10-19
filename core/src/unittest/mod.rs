pub mod setup {
    pub mod log {
        use std::sync::Once;
        static SETUP: Once = Once::new();
        pub fn configure() {
            configure_level(log::LevelFilter::Trace)
        }
        pub fn configure_level(level: log::LevelFilter) {
            SETUP.call_once(|| {
                use colored::*;
                use std::io::Write;
                let _ = env_logger::builder()
                    .format(|buf, record| {
                        let ts = buf.timestamp_nanos();
                        let level = match record.level() {
                            log::Level::Error => "ERROR".red(),
                            log::Level::Warn => "WARN ".yellow(),
                            log::Level::Info => "INFO ".green(),
                            log::Level::Debug => "DEBUG".blue(),
                            log::Level::Trace => "TRACE".blue(),
                        };
                        let target = record.target();
                        let args = record.args();
                        let thread = std::thread::current();
                        let id = thread.id();
                        let name = thread
                            .name()
                            .unwrap_or(format!("Thread-{id:?}").as_str())
                            .to_owned();
                        writeln!(buf, "{ts} {level} ({name}) {target} {args}")
                    })
                    // .format_timestamp_micro s()
                    .is_test(false) // disables color in the terminal
                    .filter_level(level)
                    .try_init();
            });
        }
    }
    pub mod net {
        use std::{net::TcpListener, ops::Range, time::Duration};

        pub fn find_available_port(range: Range<u16>) -> u16 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..1000 {
                let port = rng.gen_range(range.clone());
                if TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
                    return port;
                }
            }
            panic!("Unable to find an available port in range: {:?}", range);
        }

        pub fn rand_avail_addr_port() -> &'static str {
            let port = find_available_port(8_000..60_000);
            let addr = format!("0.0.0.0:{}", port).into_boxed_str();
            Box::leak(addr)
        }

        pub fn default_connect_timeout() -> Duration {
            Duration::from_millis(500)
        }
        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5
        }

        pub fn optional_find_timeout() -> Option<Duration> {
            Some(Duration::from_millis(10))
        }
    }
    pub mod data {
        pub fn random_bytes(size: usize) -> &'static [u8] {
            let data = (0..size).map(|_| rand::random::<u8>()).collect::<Vec<_>>();
            let leaked_ref: &'static [u8] = Box::leak(data.into_boxed_slice());
            leaked_ref
        }
    }
    pub mod model {
        pub const TEXT_SIZE: usize = 127;
        pub const TEST_MSG_FRAME_SIZE: usize = TEXT_SIZE + 1;
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
        use byteserde_types::prelude::*;

        #[rustfmt::skip]
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default)]
        pub struct TestCltMsgDebug {
            ty: ConstCharAscii<b'1'>,
            pub text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
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
            pub text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
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
        impl TestMsg {
            pub fn try_into_clt(self) -> TestCltMsg {
                match self {
                    Self::Clt(msg) => msg,
                    _ => panic!("Not a Clt message"),
                }
            }
            pub fn try_into_svc(self) -> TestSvcMsg {
                match self {
                    Self::Svc(msg) => msg,
                    _ => panic!("Not a Svc message"),
                }
            }
            pub fn is_clt(&self) -> bool {
                matches!(self, Self::Clt(_))
            }
            pub fn is_svc(&self) -> bool {
                matches!(self, Self::Svc(_))
            }
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

        #[cfg(test)]
        mod test {
            use super::*;
            use byteserde::size::ByteSerializedLenOf;
            // for simplicity the framer assume each message to be of fixed size, this test just to avoid mistakes
            #[test]
            fn test_msg_len() {
                assert_eq!(TestCltMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(
                    TestCltMsgLoginReq::default().byte_len(),
                    TEST_MSG_FRAME_SIZE
                );
                assert_eq!(TestSvcMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(
                    TestSvcMsgLoginAcpt::default().byte_len(),
                    TEST_MSG_FRAME_SIZE
                );
                assert_eq!(TestHBeatMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
            }
        }
    }

    pub mod framer {

        use bytes::BytesMut;

        use crate::prelude::*;

        pub use super::model::*;

        pub type TestMsgFramer = FixedSizeFramer<TEST_MSG_FRAME_SIZE>;

        #[derive(Debug, Clone, PartialEq)]
        pub struct CltTestMessenger;

        impl Framer for CltTestMessenger {
            fn get_frame_length(bytes: &mut BytesMut) -> Option<usize> {
                TestMsgFramer::get_frame_length(bytes)
            }
        }

        #[derive(Debug, Clone, PartialEq)]
        pub struct SvcTestMessenger;

        impl Framer for SvcTestMessenger {
            fn get_frame_length(bytes: &mut BytesMut) -> Option<usize> {
                TestMsgFramer::get_frame_length(bytes)
            }
        }
    }

    pub mod messenger {
        use std::io::Error;

        pub use super::framer::*;
        use byteserde::prelude::{from_slice, to_bytes_stack};

        use crate::prelude::*;

        impl Messenger for SvcTestMessenger {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;

            #[inline(always)]
            fn serialize<const MMS: usize>(msg: &Self::SendT) -> Result<([u8; MMS], usize), Error> {
                match to_bytes_stack::<MMS, Self::SendT>(msg) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e.message)),
                }
            }

            #[inline(always)]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error> {
                match from_slice::<Self::RecvT>(frame) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e.message)),
                }
            }
        }
        impl Messenger for CltTestMessenger {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
            #[inline(always)]
            fn serialize<const MMS: usize>(msg: &Self::SendT) -> Result<([u8; MMS], usize), Error> {
                match to_bytes_stack::<MMS, Self::SendT>(msg) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e.message)),
                }
            }

            #[inline(always)]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error> {
                match from_slice::<Self::RecvT>(frame) {
                    Ok(res) => Ok(res),
                    Err(e) => Err(Error::new(std::io::ErrorKind::Other, e.message)),
                }
            }
        }
    }

    // TODO remove
    pub mod messenger_old {
        pub use super::framer::CltTestMessenger;
        pub use super::framer::SvcTestMessenger;

        use crate::prelude::*;
        use crate::unittest::setup::model::*;
        impl MessengerOld for SvcTestMessenger {
            type SendT = TestSvcMsg;
            type RecvT = TestCltMsg;
        }
        impl MessengerOld for CltTestMessenger {
            type SendT = TestCltMsg;
            type RecvT = TestSvcMsg;
        }
    }
}
