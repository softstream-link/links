use crate::prelude::*;
use std::fmt::{Debug, Display};

pub mod chain;
pub mod counter;
pub mod devnull;
pub mod logger;
pub mod store;

/// Trait for callbacks that wish to get all received messages.
pub trait CallbackRecv<M: Messenger>: Display + Debug + Send + Sync + 'static {
    /// Will be called after message is received and deserialized.
    fn on_recv(&self, con_id: &ConId, msg: &M::RecvT);
}

/// Trait for callbacks that wish to get all sent messages.
pub trait CallbackSend<M: Messenger>: Display + Debug {
    /// Will be called after message is serialized and sent.
    fn on_sent(&self, con_id: &ConId, msg: &M::SendT);
}

/// Super trait for callbacks that wish to get all sent and received messages.
pub trait CallbackRecvSend<M: Messenger>: CallbackRecv<M> + CallbackSend<M> {}
