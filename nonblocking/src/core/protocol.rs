use crate::prelude::{CallbackRecvSend, Clt, Messenger};
use std::io::Error;

pub trait Protocol: Messenger + Clone {
    #[allow(unused_variables)]
    fn on_connected<P: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<P, C, MAX_MSG_SIZE>) -> Result<(), Error> {
        Ok(())
    }
}
