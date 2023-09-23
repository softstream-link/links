use std::{
    io::Error,
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackRecvSend, Messenger};

use crate::prelude_blocking::Clt;

// ----- Acceptor -----

pub trait AcceptClt<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    /// Blocking accept
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error>;
    /// Non-blocking accept
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error>;
    
    /// Will call [Self::accept_nonblocking] busywaiting untill it returns [Some(Clt)] or
    /// will return [Ok(None)] if the call to [Self::accept_nonblocking] returns [None] after the timeout
    fn accept_busywait_timeout(
        &self,
        timeout: Duration,
    ) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        let start = Instant::now();
        loop {
            match self.accept_nonblocking()? {
                Some(clt) => return Ok(Some(clt)),
                None => {
                    if start.elapsed() > timeout {
                        return Ok(None);
                    }
                }
            }
        }
    }

    /// Will call [Self::accept_nonblocking] while busywaiting untill it returns [`Some(Clt)`]
    fn accept_busywait(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        loop {
            match self.accept_nonblocking()? {
                Some(clt) => return Ok(clt),
                None => continue,
            }
        }
    }
}

// ----- Recver -----

pub trait RecvMsg<M: Messenger> {
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error>;
}

// ----- Sender -----

pub trait SendMsg<M: Messenger> {
    fn send(&mut self, msg: &mut M::SendT) -> Result<(), Error>;
}
