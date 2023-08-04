use std::fmt::{Debug, Display};

use crate::core::{ConId, Messenger};

pub mod devnull;
pub mod logger;
pub mod store;
pub mod chain;

pub trait CallbackSendRecv<M: Messenger>: Debug + Display + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvMsg);
    fn on_send(&self, con_id: &ConId, msg: &M::SendMsg);
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
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Send + Sync + 'static,
{
    fn on_event(&self, cond_id: &ConId, event: Dir<T>);
}
