pub mod framer;
pub mod messenger;

use std::io::Error;

use links_core::prelude::{CallbackRecvSend, Messenger};

use crate::prelude::Clt;

// ----- Acceptor -----

pub trait PoolAcceptClt<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    fn pool_accept(&mut self) -> Result<(), Error>;
}

pub trait AcceptClt<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    /// Blocking accept
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error>;
}

// ----- Recver -----

pub trait RecvMsg<M: Messenger> {
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error>;
}

// ----- Sender -----

pub trait SendMsg<M: Messenger> {
    fn send(&mut self, msg: &mut M::SendT) -> Result<(), Error>;
}

pub trait SendMsgNonMut<M: Messenger> {
    fn send(&mut self, msg: &M::SendT) -> Result<(), Error>;
}
