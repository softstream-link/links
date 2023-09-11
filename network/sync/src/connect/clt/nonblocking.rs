use std::{
    error::Error,
    fmt::Display,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude_nonblocking::*;

use links_network_core::prelude::{CallbackRecv, CallbackSend, ConId, MessengerNew};
use log::debug;

use crate::connect::messenger::nonblocking::{into_split_messenger, MessageRecver, MessageSender};

#[derive(Debug)]
pub struct CltSender<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<CSend>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize>
    CltSender<M, CSend, MAX_MSG_SIZE>
{
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<CSend>) -> Self {
        Self {
            msg_sender: sender,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgNonBlockingMut<M>
    for CltSender<M, CSend, MAX_MSG_SIZE>
{
    ///
    #[inline(always)]
    fn send_nonblocking(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<WriteStatus, Box<dyn Error>> {
        self.callback.on_send(&self.msg_sender.con_id, msg);
        self.msg_sender.send_nonblocking(msg)
    }
}
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendMsgBusyWaitMut<M>
    for CltSender<M, CSend, MAX_MSG_SIZE>
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
impl<M: MessengerNew, CSend: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for CltSender<M, CSend, MAX_MSG_SIZE>
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
pub struct CltRecver<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<CRecv>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize>
    CltRecver<M, CRecv, MAX_MSG_SIZE>
{
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<CRecv>) -> Self {
        Self {
            msg_recver: recver,
            callback,
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for CltRecver<M, CRecv, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Box<dyn Error>> {
        let msg = self.msg_recver.recv()?;
        match msg {
            ReadStatus::Completed(Some(msg)) => {
                self.callback.on_recv(&self.msg_recver.con_id, msg);
                Ok(ServiceLoopStatus::Continue)
            }
            ReadStatus::WouldBlock => Ok(ServiceLoopStatus::Continue),
            ReadStatus::Completed(None) => Ok(ServiceLoopStatus::Stop),
        }
    }
}
impl<M: MessengerNew, CRecv: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for CltRecver<M, CRecv, MAX_MSG_SIZE>
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

pub struct Clt<
    M: MessengerNew,
    CRecv: CallbackRecv<M>,
    CSend: CallbackSend<M>,
    const MAX_MSG_SIZE: usize,
> {
    clt_recver: CltRecver<M, CRecv, MAX_MSG_SIZE>,
    clt_sender: CltSender<M, CSend, MAX_MSG_SIZE>,
}

impl<
        M: MessengerNew,
        CRecv: CallbackRecv<M>,
        CSend: CallbackSend<M>,
        const MAX_MSG_SIZE: usize,
    > Clt<M, CRecv, CSend, MAX_MSG_SIZE>
{
    pub fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback_recv: Arc<CRecv>,
        callback_send: Arc<CSend>,
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
                    return Ok(Self::from_stream(
                        stream,
                        con_id,
                        callback_recv,
                        callback_send,
                    ));
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }

    pub(crate) fn from_stream(
        stream: TcpStream,
        con_id: ConId,
        callback_recv: Arc<CRecv>,
        callback_send: Arc<CSend>,
    ) -> Self {
        let (msg_recver, msg_sender) = into_split_messenger::<M, MAX_MSG_SIZE>(stream, con_id);
        Self {
            clt_recver: CltRecver::new(msg_recver, callback_recv),
            clt_sender: CltSender::new(msg_sender, callback_send),
        }
    }
    pub fn into_split(
        self,
    ) -> (
        CltRecver<M, CRecv, MAX_MSG_SIZE>,
        CltSender<M, CSend, MAX_MSG_SIZE>,
    ) {
        (self.clt_recver, self.clt_sender)
    }
}

impl<
        M: MessengerNew,
        CRecv: CallbackRecv<M>,
        CSend: CallbackSend<M>,
        const MAX_MSG_SIZE: usize,
    > SendMsgNonBlockingMut<M> for Clt<M, CRecv, CSend, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_nonblocking(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<WriteStatus, Box<dyn Error>> {
        self.clt_sender.send_nonblocking(msg)
    }
}
impl<
        M: MessengerNew,
        CRecv: CallbackRecv<M>,
        CSend: CallbackSend<M>,
        const MAX_MSG_SIZE: usize,
    > SendMsgBusyWaitMut<M> for Clt<M, CRecv, CSend, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_busywait(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<(), Box<dyn Error>> {
        self.clt_sender.send_busywait(msg)
    }
}
// impl<M: MessengerNew, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M> for MessageSender<M, MAX_MSG_SIZE>{
impl<
        M: MessengerNew,
        CRecv: CallbackRecv<M>,
        CSend: CallbackSend<M>,
        const MAX_MSG_SIZE: usize,
    > Display for Clt<M, CRecv, CSend, MAX_MSG_SIZE>
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
        let res = super::Clt::<_, _, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            std::time::Duration::from_millis(50),
            std::time::Duration::from_millis(10),
            callback.clone(),
            callback.clone(),
            Some("unittest"),
        );
        assert!(res.is_err());
    }
}
