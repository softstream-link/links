use std::{fmt::{Debug, Display}, time::{Instant, SystemTime}};

use crate::prelude::{MessengerOld, ConId};

use super::CallbackSendRecvOld;


#[derive(Debug, Clone, PartialEq)]
pub enum DirOld<T> {
    Recv(T),
    Send(T),
}
impl<T> DirOld<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Recv(t) => t,
            Self::Send(t) => t,
        }
    }
}

pub trait CallbackEvent<T, M: MessengerOld>: CallbackSendRecvOld<M>
where T: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync+'static
{
    fn on_event(&self, cond_id: &ConId, event: DirOld<T>);
}


#[derive(Debug, Clone, PartialEq)]
pub struct EntryOld<T> {
    pub con_id: ConId,
    pub instant: Instant,
    pub time: SystemTime,
    pub event: DirOld<T>,
}
impl<T> EntryOld<T> {
    pub fn unwrap_recv_event(self) -> T {
        match self.event {
            DirOld::Recv(t) => t,
            DirOld::Send(_) => panic!("Entry::try_into_recv: Not a Dir::Recv variant"),
        }
    }
    pub fn unwrap_send_event(self) -> T {
        match self.event {
            DirOld::Recv(_) => panic!("Entry::try_into_send: Not a Dir::Send variant"),
            DirOld::Send(t) => t,
        }
    }
}
impl<T: Debug> Display for EntryOld<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{:?}", self.con_id, self.event)
    }
}