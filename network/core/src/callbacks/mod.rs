use std::fmt::{Debug, Display};

use crate::core::Messenger;
use crate::prelude::*;

pub mod chain;
pub mod counter;
pub mod devnull;
pub mod eventstore;
pub mod logger;
pub trait CallbackSendRecv<M: Messenger>: Debug+Display+Send+Sync+'static {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT);
    fn on_send(&self, con_id: &ConId, msg: &M::SendT);
}
