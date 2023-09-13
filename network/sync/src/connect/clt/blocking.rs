use std::{
    fmt::Display,
    io::Error,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude_blocking::{
    into_split_messenger, MessageRecver, MessageSender, SendMsg, SendMsgMut,
};
use links_network_core::prelude::{CallbackRecv, CallbackSend, CallbackSendRecv, ConId, Messenger};
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
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "CltRecver<{}, {}, {}>",
            self.msg_recver.con_id, name, MAX_MSG_SIZE
        )
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
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgMut<M>
    for CltSender<M, C, MAX_MSG_SIZE>
{
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        self.callback.on_send(&self.msg_sender.con_id, msg);
        self.msg_sender.send(msg)
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for CltSender<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = std::any::type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "CltSender<{}, {}, {}>",
            self.msg_sender.con_id, name, MAX_MSG_SIZE
        )
    }
}

pub struct Clt<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> {
    clt_recver: CltRecver<M, C, MAX_MSG_SIZE>,
    clt_sender: CltSender<M, C, MAX_MSG_SIZE>,
}

impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> Clt<M, C, MAX_MSG_SIZE> {
    pub fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<C>,
        name: Option<&str>,
    ) -> Result<Self, Error> {
        assert!(timeout > retry_after);
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
        let (msg_recver, msg_sender) = into_split_messenger::<M, MAX_MSG_SIZE>(stream, con_id);
        Self {
            clt_recver: CltRecver::new(msg_recver, callback.clone()),
            clt_sender: CltSender::new(msg_sender, callback.clone()),
        }
    }
    pub fn into_split(self) -> (CltRecver<M, C, MAX_MSG_SIZE>, CltSender<M, C, MAX_MSG_SIZE>) {
        (self.clt_recver, self.clt_sender)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use super::Clt;
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TEST_MSG_FRAME_SIZE};
    use links_network_core::callbacks::logger_new::LoggerCallbackNew;
    use links_testing::unittest::setup;

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallbackNew::<TestCltMsgProtocol>::new_ref();
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            std::time::Duration::from_millis(50),
            std::time::Duration::from_millis(10),
            callback,
            Some("unittest"),
        );
        assert!(res.is_err());
    }
}
