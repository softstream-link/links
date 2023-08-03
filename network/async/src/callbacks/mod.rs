use std::fmt::{Debug, Display};

use crate::core::{ConId, Messenger};

pub mod chain;
pub mod messengerstore;
pub mod eventstore;
pub mod logger;

pub trait CallbackSendRecv<MESSENGER: Messenger>: Debug + Display + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: MESSENGER::RecvMsg);
    fn on_send(&self, con_id: &ConId, msg: &MESSENGER::SendMsg);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event<TARGET> {
    Recv(TARGET),
    Send(TARGET),
}

pub trait CallbackEvent<TARGET, MESSENGER: Messenger>: CallbackSendRecv<MESSENGER>
where
    TARGET: From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Send + Sync + 'static,
{
    fn on_event(&self, cond_id: &ConId, event: Event<TARGET>);
}
