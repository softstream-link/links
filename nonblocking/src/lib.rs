#![doc = include_str!("./../../readme.md")]
#![doc = include_str!("./../../readme/src/nonblocking/100_nonblocking.md")]
#![doc = include_str!("./../../readme/src/nonblocking/110_data_model.md")]
#![doc = include_str!("./../../readme/src/nonblocking/120_protocol.md")]
#![doc = include_str!("./../../readme/src/nonblocking/121_framer.md")]
#![doc = include_str!("./../../readme/src/nonblocking/122_messenger.md")]
#![doc = include_str!("./../../readme/src/nonblocking/123_launching_svc.md")]
#![doc = include_str!("./../../readme/src/nonblocking/124_connecting_clt.md")]


pub mod connect;
pub mod core;
pub mod prelude;

#[cfg(feature = "unittest")]
pub mod unittest;
