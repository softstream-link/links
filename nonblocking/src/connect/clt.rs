use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::prelude::{into_split_messenger, CallbackRecv, CallbackRecvSend, CallbackSend, ConId, MessageRecver, MessageSender, Messenger, PollEventStatus, PollRecv, Protocol, RecvNonBlocking, RecvStatus, SendNonBlocking, SendNonBlockingNonMut, SendStatus};
use links_core::asserted_short_name;
use log::debug;

/// An abstraction over a [MessageRecver] that calls a [CallbackRecv] on every message being processed by [CltRecver].
/// It is designed to work in a thread that is different from [CltSender]
#[derive(Debug)]
pub struct CltRecver<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self { msg_recver: recver, callback }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> RecvNonBlocking<M> for CltRecver<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<M::RecvT>, Error> {
        match self.msg_recver.recv()? {
            RecvStatus::Completed(Some(msg)) => {
                self.callback.on_recv(&self.msg_recver.frm_reader.con_id, &msg);
                Ok(RecvStatus::Completed(Some(msg)))
            }
            RecvStatus::Completed(None) => Ok(RecvStatus::Completed(None)),
            RecvStatus::WouldBlock => Ok(RecvStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> PollRecv for CltRecver<M, C, MAX_MSG_SIZE> {
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        Box::new(&mut self.msg_recver.frm_reader.stream_reader)
    }
    fn on_readable_event(&mut self) -> Result<PollEventStatus, Error> {
        use RecvStatus::*;
        match self.recv()? {
            Completed(Some(_)) => Ok(PollEventStatus::Completed),
            WouldBlock => Ok(PollEventStatus::WouldBlock),
            Completed(None) => Ok(PollEventStatus::Terminate),
        }
    }
    fn con_id(&self) -> &ConId {
        &self.msg_recver.frm_reader.con_id
    }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display for CltRecver<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltRecver", Self), self.msg_recver.frm_reader.con_id, recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageSender] that calls a [CallbackSend] on every message sent by [CltSender].
/// It is designed to work in a thread that is different from [CltRecver]
#[derive(Debug)]
pub struct CltSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSender<M, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<C>) -> Self {
        Self { msg_sender: sender, callback }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendNonBlocking<M> for CltSender<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.callback.on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send(msg) {
            Ok(SendStatus::Completed) => {
                self.callback.on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(SendStatus::Completed)
            }
            Ok(SendStatus::WouldBlock) => {
                self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &ErrorKind::WouldBlock.into());
                Ok(SendStatus::WouldBlock)
            }
            Err(e) => {
                self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &e);
                Err(e)
            }
        }
    }
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <M as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        self.callback.on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send_busywait_timeout(msg, timeout) {
            Ok(SendStatus::Completed) => {
                self.callback.on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(SendStatus::Completed)
            }
            Ok(SendStatus::WouldBlock) => {
                self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &ErrorKind::WouldBlock.into());
                Ok(SendStatus::WouldBlock)
            }
            Err(e) => {
                self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &e);
                Err(e)
            }
        }
    }
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<(), Error> {
        self.callback.on_send(&self.msg_sender.frm_writer.con_id, msg);

        match self.msg_sender.send_busywait(msg) {
            Ok(()) => {
                self.callback.on_sent(&self.msg_sender.frm_writer.con_id, msg);
                Ok(())
            }
            Err(e) => {
                self.callback.on_fail(&self.msg_sender.frm_writer.con_id, msg, &e);
                Err(e)
            }
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display for CltSender<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltSender", Self), self.msg_sender.frm_writer.con_id, recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageRecver] and [MessageSender] that calls a respective callback functions on every
/// message being processed by internal [CltRecver] and [CltSender].
/// It is designed to work in a single thread. To split out [CltRecver] and [CltSender] use [Clt::into_split]
///
/// # Example
/// ```
/// use links_nonblocking::{prelude::*, unittest::setup::protocol::CltTestProtocolAuth};
/// use links_core::unittest::setup::{self, framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE}, model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg}};
/// use std::time::Duration;
///
/// let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
///         setup::net::rand_avail_addr_port(), // "127.0.0.1:8080",
///         Duration::from_millis(100),
///         Duration::from_millis(10),
///         DevNullCallback::default().into(),
///         Some(CltTestProtocolAuth::new_ref()),
///         Some("unittest"),
///     );
///
/// if res.is_ok() {
///
///     // Not Split for use in single thread
///     let mut clt = res.unwrap();
///     clt.send_busywait(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"))).unwrap();
///     let msg: SvcTestMsg = clt.recv_busywait().unwrap().unwrap();
///     
///     // Split for use in different threads
///     let (mut clt_recver, mut clt_sender) = clt.into_split();
///     clt_sender.send_busywait(&mut CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"))).unwrap();
///     let msg: SvcTestMsg = clt_recver.recv_busywait().unwrap().unwrap();
///
///     drop(clt_recver);
///     drop(clt_sender);
///     
/// }
/// ```
#[derive(Debug)]
pub struct Clt<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    clt_recver: CltRecver<P, C, MAX_MSG_SIZE>,
    clt_sender: CltSender<P, C, MAX_MSG_SIZE>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Clt<P, C, MAX_MSG_SIZE> {
    pub fn connect(addr: &str, timeout: Duration, retry_after: Duration, callback: Arc<C>, protocol: Option<Arc<P>>, name: Option<&str>) -> Result<Self, Error> {
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
                    let clt = Self::from_stream(stream, con_id, callback, protocol)?;
                    return Ok(clt);
                }
            }
        }
        let msg = format!("{:?} connect timeout: {:?}", con_id, timeout);
        Err(Error::new(std::io::ErrorKind::TimedOut, msg))
    }

    pub(crate) fn from_stream(stream: TcpStream, con_id: ConId, callback: Arc<C>, protocol: Option<Arc<P>>) -> Result<Self, Error> {
        let (msg_recver, msg_sender) = into_split_messenger::<P, MAX_MSG_SIZE>(con_id, stream);
        let mut clt = Self {
            clt_recver: CltRecver::new(msg_recver, callback.clone()),
            clt_sender: CltSender::new(msg_sender, callback.clone()),
        };
        if let Some(protocol) = protocol {
            protocol.on_connected(&mut clt)?;
        }
        Ok(clt)
    }
    pub fn con_id(&self) -> &ConId {
        &self.clt_recver.msg_recver.frm_reader.con_id
    }
    pub fn into_split(self) -> (CltRecver<P, C, MAX_MSG_SIZE>, CltSender<P, C, MAX_MSG_SIZE>) {
        (self.clt_recver, self.clt_sender)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clt_sender.send(msg)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        self.clt_recver.recv()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for Clt<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}, {}>", asserted_short_name!("Clt", Self), self.clt_recver, self.clt_sender)
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use super::Clt;
    use crate::unittest::setup::protocol::CltTestProtocolAuth;
    use links_core::callbacks::logger::LoggerCallback;
    use links_core::unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE};

    #[test]
    fn test_clt_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        let callback = LoggerCallback::new_ref();
        let protocol = CltTestProtocolAuth::new_ref();
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, Some(protocol), Some("unittest"));
        assert!(res.is_err());
    }
}
