use std::error::Error;

use crate::core::messenger::MessengerNew;

pub trait SendMsgBlocking<M: MessengerNew> {
    fn send(&mut self, msg: &M::SendT) -> Result<(), Box<dyn Error>>;
}

pub trait RecvMsgBlocking<M: MessengerNew> {
    fn recv(&mut self) -> Result<Option<M::RecvT>, Box<dyn Error>>;
}
