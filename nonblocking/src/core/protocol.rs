use crate::prelude::{CallbackRecvSend, Clt, Messenger};
use std::io::Error;

pub trait Protocol: Messenger {
    #[allow(unused_variables)]
    fn on_connected<M: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
        Ok(())
    }
}
