use std::{
    fmt::Display,
    io::Error,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude::{into_split_messenger, MessageRecver, MessageSender, RecvMsg, SendMsg, SendMsgNonMut};
use links_core::prelude::{CallbackRecv, CallbackRecvSend, CallbackSend, ConId, Messenger};
use log::debug;

#[derive(Debug)]
pub struct CltRecver<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_recver: recver,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsg<M> for CltRecver<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error> {
        match self.msg_recver.recv()? {
            Some(msg) => {
                self.callback.on_recv(&self.msg_recver.frm_reader.con_id, &msg);
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display for CltRecver<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "CltRecver<{}, {}, {}>", self.msg_recver.frm_reader.con_id, name, MAX_MSG_SIZE)
    }
}

#[derive(Debug)]
pub struct CltSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSender<M, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_sender: sender,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsg<M> for CltSender<M, C, MAX_MSG_SIZE> {
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        // self.callback.on_send(&self.msg_sender.frm_writer.con_id, msg);
        match self.msg_sender.send(msg) {
            Ok(()) => {
                self.callback.on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(())
            }
            Err(e) => {
                // self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &e);
                Err(e)
            }
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display for CltSender<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "CltSender<{}, {}, {}>", self.msg_sender.frm_writer.con_id, name, MAX_MSG_SIZE)
    }
}

#[derive(Debug)]
pub struct Clt<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    clt_recver: CltRecver<M, C, MAX_MSG_SIZE>,
    clt_sender: CltSender<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Clt<M, C, MAX_MSG_SIZE> {
    pub fn connect(addr: &str, timeout: Duration, retry_after: Duration, callback: Arc<C>, name: Option<&str>) -> Result<Self, Error> {
        assert!(timeout > retry_after, "timeout: {:?}, retry_after: {:?}", timeout, retry_after);
        let now = Instant::now();
        let con_id = ConId::clt(name, None, addr);
        while now.elapsed() < timeout {
            match TcpStream::connect(addr) {
                Err(e) => {
                    sleep(retry_after);
                    debug!("{} connection failed. e: {:?}", con_id, e);
                    continue;
                }
                Ok(stream) => {
                    return Ok(Self::from_stream(stream, con_id, callback));
                }
            }
        }

        let msg = format!("{:?} connect timeout: {:?}", con_id, timeout);
        Err(Error::new(std::io::ErrorKind::TimedOut, msg))
    }

    pub(crate) fn from_stream(stream: TcpStream, con_id: ConId, callback: Arc<C>) -> Self {
        let (msg_recver, msg_sender) = into_split_messenger::<M, MAX_MSG_SIZE>(con_id, stream);
        Self {
            clt_recver: CltRecver::new(msg_recver, callback.clone()),
            clt_sender: CltSender::new(msg_sender, callback.clone()),
        }
    }
    pub fn into_split(self) -> (CltRecver<M, C, MAX_MSG_SIZE>, CltSender<M, C, MAX_MSG_SIZE>) {
        (self.clt_recver, self.clt_sender)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsg<M> for Clt<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        self.clt_sender.send(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsg<M> for Clt<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error> {
        self.clt_recver.recv()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display for Clt<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "Clt<{}, {}, {}>", self.clt_recver.msg_recver.frm_reader.con_id, name, MAX_MSG_SIZE)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use super::Clt;

    use links_core::{
        prelude::LoggerCallback,
        unittest::setup::{
            self,
            framer::{CltTestMessenger, TEST_MSG_FRAME_SIZE},
        },
    };

    use log::info;

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::<CltTestMessenger>::new_ref();
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, std::time::Duration::from_millis(50), std::time::Duration::from_millis(10), callback, Some("unittest"));
        info!("res: {:?}", res);
        assert!(res.is_err());
    }
}
