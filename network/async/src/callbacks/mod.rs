use std::fmt::{Debug, Display};

use crate::core::{conid::ConId, Messenger};

pub mod chain;
pub mod devnull;
pub mod eventstore;
pub mod logger;

pub trait CallbackSendRecv<M: Messenger>: Debug + Display + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT);
    fn on_send(&self, con_id: &ConId, msg: &M::SendT);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Dir<T> {
    Recv(T),
    Send(T),
}
impl<T> Dir<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Recv(t) => t,
            Self::Send(t) => t,
        }
    }
}

pub trait CallbackEvent<T, M: Messenger>: CallbackSendRecv<M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Send + Sync + 'static,
{
    fn on_event(&self, cond_id: &ConId, event: Dir<T>);
}
