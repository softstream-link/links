pub mod clt;
pub mod poll;
pub mod pool;
pub mod svc;

use self::poll::{PollHandlerDynamic, SpawnedPollHandlerDynamic};
use crate::prelude::Timer;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_POLL_HANDLER: SpawnedPollHandlerDynamic = PollHandlerDynamic::default().into_spawned_handler("Default-RecvPollHandler-Thread");
    pub static ref DEFAULT_HBEAT_HANDLER: Timer = Timer::new("Default-HeartbeatHandler-Thread");
}
