use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude_nonblocking::{
    into_split_messenger, CallbackRecv, CallbackRecvSend, CallbackSend, ConId, MessageRecver,
    MessageSender, Messenger, NonBlockingServiceLoop, RecvMsgNonBlocking, RecvStatus,
    SendMsgNonBlocking, SendMsgNonBlockingNonMut, SendStatus, ServiceLoopStatus,
};
use log::debug;

#[derive(Debug)]
pub struct CltRecver<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_recver: recver,
            callback,
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        match self.msg_recver.recv_nonblocking()? {
            RecvStatus::Completed(Some(msg)) => {
                self.callback
                    .on_recv(&self.msg_recver.frm_reader.con_id, &msg);
                Ok(RecvStatus::Completed(Some(msg)))
            }
            RecvStatus::Completed(None) => Ok(RecvStatus::Completed(None)),
            RecvStatus::WouldBlock => Ok(RecvStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        match self.recv_nonblocking()? {
            RecvStatus::WouldBlock | RecvStatus::Completed(Some(_)) => {
                Ok(ServiceLoopStatus::Continue)
            }
            RecvStatus::Completed(None) => Ok(ServiceLoopStatus::Stop),
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
            self.msg_recver.frm_reader.con_id, name, MAX_MSG_SIZE
        )
    }
}

#[derive(Debug)]
pub struct CltSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    pub(crate) msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSender<M, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_sender: sender,
            callback,
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for CltSender<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.callback
            .on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send_nonblocking(msg) {
            Ok(SendStatus::Completed) => {
                self.callback
                    .on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(SendStatus::Completed)
            }
            Ok(SendStatus::WouldBlock) => {
                self.callback.on_fail(
                    &self.msg_sender.frm_writer.con_id,
                    msg,
                    &ErrorKind::WouldBlock.into(),
                );
                Ok(SendStatus::WouldBlock)
            }
            Err(e) => {
                self.callback
                    .on_fail(&self.msg_sender.frm_writer.con_id, msg, &e);
                Err(e)
            }
        }
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
            self.msg_sender.frm_writer.con_id, name, MAX_MSG_SIZE
        )
    }
}

pub struct Clt<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> {
    clt_recver: CltRecver<M, C, MAX_MSG_SIZE>,
    clt_sender: CltSender<M, C, MAX_MSG_SIZE>,
}

impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Clt<M, C, MAX_MSG_SIZE> {
    pub fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<C>,
        name: Option<&str>,
    ) -> Result<Self, Error> {
        assert!(
            timeout > retry_after,
            "timeout: {:?}, retry_after: {:?}",
            timeout,
            retry_after
        );
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
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clt_sender.send_nonblocking(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.clt_recver.recv_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Clt<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clt<{}, {}>", self.clt_recver, self.clt_sender)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use super::Clt;
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TEST_MSG_FRAME_SIZE};
    use links_network_core::callbacks::logger_new::LoggerCallback;
    use links_testing::unittest::setup;

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::<TestCltMsgProtocol>::new_ref();
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            callback,
            Some("unittest"),
        );
        assert!(res.is_err());
    }
}
