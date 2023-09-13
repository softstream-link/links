use std::io::Error;

use links_network_core::prelude::{CallbackSendRecv, Messenger};

use crate::prelude_blocking::Clt;

// ----- Acceptor -----

pub trait AcceptClt<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> {
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error>;
}

// ----- Recver -----

pub trait RecvMsg<M: Messenger> {
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error>;
}

// ----- Sender -----

pub trait SendMsg<M: Messenger> {
    fn send(&mut self, msg: &M::SendT) -> Result<(), Error>;
}

pub trait SendMsgMut<M: Messenger> {
    fn send(&mut self, msg: &mut M::SendT) -> Result<(), Error>;
}
