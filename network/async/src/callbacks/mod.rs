use std::fmt::{Debug, Display};

use crate::core::{ConId, Messenger};

pub mod chain;
pub mod eventlog;
pub mod eventlog_into;
pub mod logger;

pub trait CallbackSendRecv<MESSENGER: Messenger>: Debug + Display + Send + Sync + 'static {
    fn on_recv(&self, con_id: &ConId, msg: MESSENGER::RecvMsg);
    fn on_send(&self, con_id: &ConId, msg: &MESSENGER::SendMsg);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event<INTO, MESSENGER: Messenger>
where
    INTO: From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg>,
{
    Recv(INTO),
    Send(INTO),
    _Phantom(std::marker::PhantomData<MESSENGER>),
}

pub trait CallbackEvent<INTO, MESSENGER: Messenger>: CallbackSendRecv<MESSENGER>
where
    INTO: From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Send + Sync + 'static,
{
    fn on_event(&self, cond_id: &ConId, event: Event<INTO, MESSENGER>);
}
