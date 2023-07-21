use std::fmt::Debug;

use crate::{Messenger, ConId};

pub mod logger;
pub mod eventlog;
pub mod chain;

pub trait Callback<MESSENGER: Messenger>: Debug + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: MESSENGER::Message);
    fn on_send(&self, con_id: &ConId, msg: &MESSENGER::Message);
}