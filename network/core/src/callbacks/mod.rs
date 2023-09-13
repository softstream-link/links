use std::fmt::{Debug, Display};

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

pub trait CallbackRecv<M: Messenger>: Debug {
    #[allow(unused_variables)]
    fn on_recv(&self, con_id: &ConId, msg: &M::RecvT);
}

pub trait CallbackSend<M: Messenger>: Debug {
    #[allow(unused_variables)]
    fn on_send(&self, con_id: &ConId, msg: &mut M::SendT);
}

pub trait CallbackSendRecv<M: Messenger>: CallbackRecv<M>+CallbackSend<M> {}
