pub mod setup {
    pub mod log {
        use std::sync::Once;

        static SETUP: Once = Once::new();
        pub fn configure() {
            configure_level(log::LevelFilter::Trace)
        }
        pub fn configure_level(level: log::LevelFilter) {
            configure_level_internal(level, false)
        }
        pub fn configure_compact(level: log::LevelFilter) {
            configure_level_internal(level, true)
        }
        fn configure_level_internal(level: log::LevelFilter, compact: bool) {
            SETUP.call_once(|| {
                use colored::*;
                use std::io::Write;
                if !compact {
                    let _ = env_logger::builder()
                        .filter_level(level)
                        .format(|buf, record| {
                            static mut MAX_THREAD_WITH: usize = 20;
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
                            let name = thread.name().unwrap_or(format!("Thread-{id:?}").as_str()).to_owned();
                            unsafe { MAX_THREAD_WITH = MAX_THREAD_WITH.max(name.len()) };
                            let name = format!("{: <0width$}", name, width = unsafe { MAX_THREAD_WITH });
                            writeln!(buf, "{ts} {level} ({name}) {target} {args}")
                        })
                        // .format_timestamp_micro s()
                        .is_test(false) // disables color in the terminal
                        .try_init();
                } else {
                    let _ = env_logger::builder()
                        .filter_level(level)
                        .format(|buf, record| {
                            let ts = buf.timestamp_nanos();
                            let level = match record.level() {
                                log::Level::Error => "ERROR".red(),
                                log::Level::Warn => "WARN ".yellow(),
                                log::Level::Info => "INFO ".green(),
                                log::Level::Debug => "DEBUG".blue(),
                                log::Level::Trace => "TRACE".blue(),
                            };
                            let pkg_name = record.target().to_owned();
                            let split = pkg_name.split("::").map(|x| x.to_owned()).collect::<Vec<_>>();
                            let first = split.first().unwrap();
                            let mut it = split.iter().rev();
                            let _ = it.next();
                            let last = it.next().unwrap();
                            let args = record.args();

                            let thread = std::thread::current();
                            let id = thread.id();
                            let mut name = thread.name().unwrap_or(format!("Thread-{id:?}").as_str()).to_owned();
                            if name.contains("::") {
                                name = "main-Thread".to_owned();
                            }

                            writeln!(buf, "{ts} {level} ({name}) {first}::*::{last} {args}")
                        })
                        // .format_timestamp_micro s()
                        .is_test(false) // disables color in the terminal
                        .try_init();
                }
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
            let port = find_available_port(2_000..65_000);
            let addr = format!("127.0.0.1:{}", port).into_boxed_str();
            Box::leak(addr)
        }

        pub fn default_connect_timeout() -> Duration {
            Duration::from_millis(1000) // 1000 mil | 1 sec
        }

        pub fn default_connect_retry_after() -> Duration {
            default_connect_timeout() / 5 // 0.2 sec
        }

        pub fn default_io_timeout() -> Duration {
            Duration::from_micros(500) // 500 mic | 0.5 mil | 0.000500 sec
        }
        pub fn default_find_timeout() -> Duration {
            Duration::from_millis(200) // 200 mil  | 0.002 sec
        }
        pub fn default_optional_find_timeout() -> Option<Duration> {
            Some(default_find_timeout())
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
        use serde::{Deserialize, Serialize};

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct CltTestMsgDebug {
            ty: ConstCharAscii<b'1'>,
            pub text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl CltTestMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct CltTestMsgLoginReq {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct CltTestMsgPing {
            pub ty: ConstCharAscii<b'P'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct SvcTestMsgDebug {
            ty: ConstCharAscii<b'2'>,
            pub text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl SvcTestMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct SvcTestMsgLoginAcpt {
            pub ty: ConstCharAscii<b'L'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct SvcTestMsgPong {
            pub ty: ConstCharAscii<b'P'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct SvcTestMsgFinal {
            pub ty: ConstCharAscii<b'F'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
        pub struct UniTestHBeatMsgDebug {
            ty: ConstCharAscii<b'H'>,
            text: StringAsciiFixed<TEXT_SIZE, b' ', true>,
        }
        impl UniTestHBeatMsgDebug {
            pub fn new(text: &[u8]) -> Self {
                Self {
                    ty: Default::default(),
                    text: StringAsciiFixed::from(text),
                }
            }
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Serialize, Deserialize)]
        #[byteserde(peek(0, 1))]
        pub enum CltTestMsg {
            #[byteserde(eq(&[b'1']))]
            Dbg(CltTestMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Login(CltTestMsgLoginReq),
            #[byteserde(eq(&[b'H']))]
            HBeat(UniTestHBeatMsgDebug),
            #[byteserde(eq(&[b'P']))]
            Ping(CltTestMsgPing),
        }
        impl From<CltTestMsgDebug> for CltTestMsg {
            fn from(msg: CltTestMsgDebug) -> Self {
                Self::Dbg(msg)
            }
        }
        impl From<CltTestMsgLoginReq> for CltTestMsg {
            fn from(value: CltTestMsgLoginReq) -> Self {
                Self::Login(value)
            }
        }
        impl From<UniTestHBeatMsgDebug> for CltTestMsg {
            fn from(value: UniTestHBeatMsgDebug) -> Self {
                Self::HBeat(value)
            }
        }
        impl From<CltTestMsgPing> for CltTestMsg {
            fn from(value: CltTestMsgPing) -> Self {
                Self::Ping(value)
            }
        }

        #[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug, Serialize, Deserialize)]
        #[byteserde(peek(0, 1))]
        pub enum SvcTestMsg {
            #[byteserde(eq(&[b'2']))]
            Dbg(SvcTestMsgDebug),
            #[byteserde(eq(&[b'L']))]
            Accept(SvcTestMsgLoginAcpt),
            #[byteserde(eq(&[b'H']))]
            HBeat(UniTestHBeatMsgDebug),
            #[byteserde(eq(&[b'P']))]
            Pong(SvcTestMsgPong),
            #[byteserde(eq(&[b'F']))]
            Final(SvcTestMsgFinal),
        }
        impl From<SvcTestMsgDebug> for SvcTestMsg {
            fn from(msg: SvcTestMsgDebug) -> Self {
                Self::Dbg(msg)
            }
        }
        impl From<SvcTestMsgLoginAcpt> for SvcTestMsg {
            fn from(value: SvcTestMsgLoginAcpt) -> Self {
                Self::Accept(value)
            }
        }
        impl From<UniTestHBeatMsgDebug> for SvcTestMsg {
            fn from(value: UniTestHBeatMsgDebug) -> Self {
                Self::HBeat(value)
            }
        }
        impl From<SvcTestMsgPong> for SvcTestMsg {
            fn from(value: SvcTestMsgPong) -> Self {
                Self::Pong(value)
            }
        }
        impl From<SvcTestMsgFinal> for SvcTestMsg {
            fn from(value: SvcTestMsgFinal) -> Self {
                Self::Final(value)
            }
        }

        #[derive(PartialEq, Clone, Debug)]
        pub enum UniTestMsg {
            Clt(CltTestMsg),
            Svc(SvcTestMsg),
        }
        impl UniTestMsg {
            pub fn try_into_clt(self) -> CltTestMsg {
                match self {
                    Self::Clt(msg) => msg,
                    _ => panic!("Not a Clt message"),
                }
            }
            pub fn try_into_svc(self) -> SvcTestMsg {
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
        impl From<&CltTestMsg> for UniTestMsg {
            fn from(msg: &CltTestMsg) -> Self {
                Self::Clt(msg.clone())
            }
        }
        impl From<&SvcTestMsg> for UniTestMsg {
            fn from(msg: &SvcTestMsg) -> Self {
                Self::Svc(msg.clone())
            }
        }

        #[cfg(test)]
        mod test {
            use super::*;
            use byteserde::size::ByteSerializedLenOf;
            // for simplicity the framer assume each message to be of fixed size, this test just to avoid mistakes
            #[test]
            fn test_msg_len() {
                assert_eq!(CltTestMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(CltTestMsgLoginReq::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(SvcTestMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(SvcTestMsgLoginAcpt::default().byte_len(), TEST_MSG_FRAME_SIZE);
                assert_eq!(UniTestHBeatMsgDebug::default().byte_len(), TEST_MSG_FRAME_SIZE);
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
            fn get_frame_length(bytes: &BytesMut) -> Option<usize> {
                TestMsgFramer::get_frame_length(bytes)
            }
        }
        #[derive(Debug, Clone, PartialEq)]
        pub struct SvcTestMessenger;
        impl Framer for SvcTestMessenger {
            fn get_frame_length(bytes: &BytesMut) -> Option<usize> {
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
            type SendT = SvcTestMsg;
            type RecvT = CltTestMsg;

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
            type SendT = CltTestMsg;
            type RecvT = SvcTestMsg;
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
}
