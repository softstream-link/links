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
    pub mod callbacks{
        use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack};
        use byteserde_types::prelude::*;

        use crate::Messenger;
    
        #[derive(ByteSerializeStack, ByteDeserializeSlice, Debug, Clone, PartialEq)]
        pub struct PayLoad {
            text: StringAsciiFixed<10, b' ', true>,
        }
        impl PayLoad{
            pub fn new(text: &[u8]) -> Self {
                Self {
                    text: StringAsciiFixed::from(text),
                }
            }
        }
    
        #[derive(Debug, Clone, PartialEq)]
        pub struct MessengerImpl;
        impl Messenger for MessengerImpl {
            type Message = PayLoad;
        }
    }
}
