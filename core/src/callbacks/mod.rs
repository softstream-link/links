use crate::core::MessengerOld;
use crate::prelude::*;
use std::fmt::{Debug, Display};

pub mod chain;
pub mod counter;
pub mod devnull;
pub mod logger;
pub mod store;

pub mod chain_old;
pub mod counter_old;
pub mod devnull_old;
pub mod eventstore_old;
pub mod logger_old;

pub trait CallbackSendRecvOld<M: MessengerOld>: Debug + Display + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT);
    fn on_send(&self, con_id: &ConId, msg: &M::SendT);
}

pub trait CallbackRecv<M: Messenger>: Display + Debug + Send + Sync + 'static {
    /// Will be called after message is received and deserialized.
    fn on_recv(&self, con_id: &ConId, msg: &M::RecvT);
}

#[allow(unused_variables)]
pub trait CallbackSend<M: Messenger>: Display + Debug {
    /// Will be called after message is serialized and sent.
    fn on_sent(&self, con_id: &ConId, msg: &M::SendT);
}

pub trait CallbackRecvSend<M: Messenger>: CallbackRecv<M> + CallbackSend<M> {}
