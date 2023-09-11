use std::{
    error::Error,
    fmt::Display,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude_nonblocking::*;

use links_network_core::{
    callbacks::CallbackSendRecvNew,
    prelude::{CallbackRecv, CallbackSend, ConId, MessengerNew},
};
use log::debug;

use crate::connect::messenger::nonblocking::{into_split_messenger, MessageRecver, MessageSender};

#[derive(Debug)]
pub struct CltSender<M: MessengerNew, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSender<M, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_sender: sender,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: MessengerNew, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlockingMut<M>
    for CltSender<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<WriteStatus, Box<dyn Error>> {
        self.callback.on_send(&self.msg_sender.con_id, msg);
        self.msg_sender.send_nonblocking(msg)
    }
}
impl<M: MessengerNew, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgBusyWaitMut<M>
    for CltSender<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_busywait(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<(), Box<dyn Error>> {
        self.callback.on_send(&self.msg_sender.con_id, msg);
        self.msg_sender.send_busywait(msg)
    }
}
impl<M: MessengerNew, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
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

#[derive(Debug)]
pub struct CltRecver<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self {
            msg_recver: recver,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>> {
        match self.msg_recver.recv_nonblocking()? {
            ReadStatus::Completed(Some(msg)) => {
                self.callback.on_recv(&self.msg_recver.con_id, &msg);
                Ok(ReadStatus::Completed(Some(msg)))
            }
            ReadStatus::Completed(None) => Ok(ReadStatus::Completed(None)),
            ReadStatus::WouldBlock => Ok(ReadStatus::WouldBlock),
        }
    }
}
impl<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvMsgBusyWait<M>
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_busywait(&mut self) -> Result<Option<<M as MessengerNew>::RecvT>, Box<dyn Error>> {
        match self.msg_recver.recv_busywait()?{
            Some(msg) => {
                self.callback.on_recv(&self.msg_recver.con_id, &msg);
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}
impl<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        match self.msg_recver.recv_nonblocking()? {
            ReadStatus::WouldBlock | ReadStatus::Completed(Some(_)) => {
                Ok(ServiceLoopStatus::Continue)
            }
            ReadStatus::Completed(None) => Ok(ServiceLoopStatus::Stop),
        }
    }
}
impl<M: MessengerNew, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
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

pub struct Clt<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> {
    clt_recver: CltRecver<M, C, MAX_MSG_SIZE>,
    clt_sender: CltSender<M, C, MAX_MSG_SIZE>,
}

impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize>
    Clt<M, C, MAX_MSG_SIZE>
{
    pub fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<C>,
        name: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
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
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
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

impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlockingMut<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<WriteStatus, Box<dyn Error>> {
        self.clt_sender.send_nonblocking(msg)
    }
}
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> SendMsgBusyWaitMut<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_busywait(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<(), Box<dyn Error>> {
        self.clt_sender.send_busywait(msg)
    }
}
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(
        &mut self,
    ) -> Result<ReadStatus<<M as MessengerNew>::RecvT>, Box<dyn Error>> {
        self.clt_recver.recv_nonblocking()
    }
}
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> RecvMsgBusyWait<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_busywait(&mut self) -> Result<Option<<M as MessengerNew>::RecvT>, Box<dyn Error>> {
        self.clt_recver.recv_busywait()
    }
}

// impl<M: MessengerNew, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M> for MessageSender<M, MAX_MSG_SIZE>{
impl<M: MessengerNew, C: CallbackSendRecvNew<M>, const MAX_MSG_SIZE: usize> Display
    for Clt<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Clt<{}, {}>", self.clt_recver, self.clt_sender)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TEST_MSG_FRAME_SIZE};

    use links_network_core::callbacks::logger_new::LoggerCallbackNew;
    use links_testing::unittest::setup;

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallbackNew::<TestCltMsgProtocol>::new_ref();
        let res = super::Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            std::time::Duration::from_millis(50),
            std::time::Duration::from_millis(10),
            callback,
            Some("unittest"),
        );
        assert!(res.is_err());
    }
}
