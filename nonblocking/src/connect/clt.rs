use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude::{
    into_split_messenger, CallbackRecv, CallbackRecvSend, CallbackSend, ConId, MessageRecver,
    MessageSender, Messenger, NonBlockingServiceLoop, RecvNonBlocking, RecvStatus,
    SendNonBlocking, SendNonBlockingNonMut, SendStatus, ServiceLoopStatus,
};
use links_core::asserted_short_name;
use log::debug;

/// An abstraction over a [MessageRecver] that calls a [CallbackRecv] on every message being processed by [CltRecver].
/// It is designed to work in a thread that is different from [CltSender]
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
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvNonBlocking<M>
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        match self.msg_recver.recv()? {
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
        use RecvStatus::*;
        match self.recv()? {
            Completed(Some(_)) => Ok(ServiceLoopStatus::Completed),
            WouldBlock => Ok(ServiceLoopStatus::WouldBlock),
            Completed(None) => Ok(ServiceLoopStatus::Terminate),
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for CltRecver<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msger_name = std::any::type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{}<{}, {}, {}>",
            asserted_short_name!("CltRecver", Self),
            self.msg_recver.frm_reader.con_id,
            msger_name,
            MAX_MSG_SIZE
        )
    }
}

/// An abstraction over a [MessageSender] that calls a [CallbackSend] on every message sent by [CltSender].
/// It is designed to work in a thread that is different from [CltRecver]
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
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendNonBlocking<M>
    for CltSender<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.callback
            .on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send(msg) {
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
    #[inline(always)]
    fn send_busywait_timeout(
        &mut self,
        msg: &mut <M as Messenger>::SendT,
        timeout: Duration,
    ) -> Result<SendStatus, Error> {
        self.callback
            .on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send_busywait_timeout(msg, timeout) {
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
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        self.callback
            .on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send_busywait(msg) {
            Ok(()) => {
                self.callback
                    .on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(())
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
        let msger_name = std::any::type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{}<{}, {}, {}>",
            asserted_short_name!("CltSender", Self),
            self.msg_sender.frm_writer.con_id,
            msger_name,
            MAX_MSG_SIZE
        )
    }
}

/// An abstraction over a [MessageRecver] and [MessageSender] that calls a respective callback functions on every
/// message being processed by internal [CltRecver] and [CltSender].
/// It is designed to work in a single thread. To split out [CltRecver] and [CltSender] use [Clt::into_split]
///
/// # Example
/// ```
/// use links_nonblocking::prelude::*;
/// use links_core::unittest::setup::{framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE}, model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg}};
/// use std::time::Duration;
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///         "127.0.0.1:8080",
///         Duration::from_millis(100),
///         Duration::from_millis(10),
///         DevNullCallback::<CltTestMessenger>::default().into(),
///         Some("unittest"),
///     );
///
/// if res.is_ok() {
///
///     // Not Split for use in single thread
///     let mut clt = res.unwrap();
///     clt.send_busywait(&mut TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"))).unwrap();
///     let msg: TestSvcMsg = clt.recv_busywait().unwrap().unwrap();
///     
///     // Split for use in different threads
///     let (mut clt_recver, mut clt_sender) = clt.into_split();
///     clt_sender.send_busywait(&mut TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"))).unwrap();
///     let msg: TestSvcMsg = clt_recver.recv_busywait().unwrap().unwrap();
///
///     drop(clt_recver);
///     drop(clt_sender);
///     
/// }
/// ```
#[derive(Debug)]
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
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> SendNonBlocking<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clt_sender.send(msg)
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> RecvNonBlocking<M>
    for Clt<M, C, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<M as Messenger>::RecvT>, Error> {
        self.clt_recver.recv()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Clt<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}, {}>",
            asserted_short_name!("Clt", Self),
            self.clt_recver,
            self.clt_sender
        )
    }
}

#[cfg(test)]
mod test {
    use super::Clt;
    use links_core::callbacks::logger_new::LoggerCallback;
    use links_core::unittest::setup::{
        self,
        framer::{CltTestMessenger, TEST_MSG_FRAME_SIZE},
    };

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::<CltTestMessenger>::new_ref();
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
