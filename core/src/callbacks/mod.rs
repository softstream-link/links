use std::fmt::{Debug, Display};
use std::io::Error;

use crate::core::MessengerOld;
use crate::prelude::*;

pub mod chain;
pub mod counter;
pub mod devnull;
pub mod devnull_new;
pub mod eventstore;
pub mod logger;
pub mod logger_new;

pub trait CallbackSendRecvOld<M: MessengerOld>: Debug+Display+Send+Sync+'static {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT);
    fn on_send(&self, con_id: &ConId, msg: &M::SendT);
}

pub trait CallbackRecv<M: Messenger>: Debug+Send+Sync+'static {
    /// Will be called after message is received and deserialized.
    fn on_recv(&self, con_id: &ConId, msg: &M::RecvT);
}

#[allow(unused_variables)]
pub trait CallbackSend<M: Messenger>: Debug {
    /// Will be called before message is serialized and sent and gives you ability to modify message.
    /// Default implementation does nothing and will be optimized away, only override if you need to modify message.
    #[inline(always)]
    fn on_send(&self, con_id: &ConId, msg: &mut M::SendT) {}

    /// Will be called if there was an unrecoverable IO Error during sending.
    /// Default implementation does nothing and will be optimized away, only override if you need to handle error.
    #[inline(always)]
    fn on_fail(&self, con_id: &ConId, msg: &M::SendT, e: &Error) {}

    /// Will be called after message is serialized and sent.
    fn on_sent(&self, con_id: &ConId, msg: &M::SendT);
}

pub trait CallbackRecvSend<M: Messenger>: CallbackRecv<M>+CallbackSend<M> {}
