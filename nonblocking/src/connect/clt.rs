use crate::prelude::{into_split_messenger, CallbackRecv, CallbackRecvSend, CallbackSend, ConId, MessageRecver, MessageSender, Messenger, PollEventStatus, PollRecv, Protocol, RecvNonBlocking, RecvStatus, SendNonBlocking, SendNonBlockingNonMut, SendStatus};
use links_core::{
    asserted_short_name,
    core::{conid::ConnectionId, counters::max_connection::RemoveConnectionBarrierOnDrop},
};
use log::debug;
use std::{
    fmt::Display,
    io::Error,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

/// An abstraction over a [MessageRecver] that calls a [CallbackRecv] on every message being processed by [CltRecver].
/// It is designed to work in a thread that is different from [CltSender]
#[derive(Debug)]
pub struct CltRecver<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    msg_recver: MessageRecver<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    #[allow(dead_code)] // exists to indicate to Svc::accept that this connection no longer active when Self is dropped
    acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> CltRecver<M, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<M, MAX_MSG_SIZE>, callback: Arc<C>, acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>) -> Self {
        Self { msg_recver: recver, callback, acceptor_connection_gate }
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
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> ConnectionId for CltRecver<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.msg_recver.frm_reader.con_id
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
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display for CltRecver<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltRecver", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageSender] that calls a [CallbackSend] on every message sent by [CltSender].
/// It is designed to work in a thread that is different from [CltRecver]
#[derive(Debug)]
pub struct CltSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    msg_sender: MessageSender<M, MAX_MSG_SIZE>,
    callback: Arc<C>,
    #[allow(dead_code)] // exists to indicate to Svc::accept that this connection no longer active when Self is dropped
    acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> CltSender<M, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<M, MAX_MSG_SIZE>, callback: Arc<C>, acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>) -> Self {
        Self { msg_sender: sender, callback, acceptor_connection_gate }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SendNonBlocking<M> for CltSender<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &mut <M as Messenger>::SendT) -> Result<SendStatus, Error> {
        let res = self.msg_sender.send(msg);
        if let Ok(SendStatus::Completed) = res {
            self.callback.on_sent(&self.msg_sender.frm_writer.con_id, msg);
        }
        res
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> ConnectionId for CltSender<M, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.msg_sender.frm_writer.con_id
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display for CltSender<M, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<M::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<M::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltSender", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [CltRecver] that also supports [Protocol] callbacks.
///
/// # Note
/// A [Protocol::on_recv] callback requires a handle to a type that is bound by [SendNonBlocking] interface so this wrapper
/// holds a reference to [CltSenderWithProtocol] which internally is a [`Arc<spin::Mutex<CltSender>>`] which requires locking as clone
/// of this mutex protected [Arc] reference is in the client space.
pub struct CltRecverWithProtocol<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    clt_sender: CltSenderWithProtocol<P, C, MAX_MSG_SIZE>,
    clt_recver: CltRecver<P, C, MAX_MSG_SIZE>,
    protocol: Arc<P>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for CltRecverWithProtocol<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltRecver] and calls [Protocol::on_recv] on respective event
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::Completed;
        let res = self.clt_recver.recv();
        if let Ok(Completed(Some(ref msg))) = res {
            self.protocol.on_recv(msg, &mut self.clt_sender)?;
        }
        res
    }
    /// Delegates to [CltRecver] and calls [Protocol::on_recv] on respective event
    fn recv_busywait_timeout(&mut self, timeout: Duration) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            let status = self.clt_recver.recv()?;
            match status {
                Completed(Some(msg)) => {
                    self.protocol.on_recv(&msg, &mut self.clt_sender)?;
                    return Ok(Completed(Some(msg)));
                }
                Completed(None) => return Ok(Completed(None)),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Delegates to [CltRecver] and calls [Protocol::on_recv] on respective event
    fn recv_busywait(&mut self) -> Result<Option<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::{Completed, WouldBlock};
        loop {
            let status = self.clt_recver.recv()?;
            match status {
                Completed(Some(msg)) => {
                    self.protocol.on_recv(&msg, &mut self.clt_sender)?;
                    return Ok(Some(msg));
                }
                Completed(None) => return Ok(None),
                WouldBlock => continue,
            }
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltRecverWithProtocol<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        self.clt_recver.con_id()
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for CltRecverWithProtocol<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltRecverWithSenderRef", Self), self.clt_recver.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [CltSender] that also supports [Protocol] callbacks.
#[derive(Debug, Clone)]
pub struct CltSenderWithProtocol<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> {
    con_id: ConId, // this is a clone copy fro CltSender to avoid mutex call to id a connection
    clt_sender: Arc<spin::Mutex<CltSender<P, C, MAX_MSG_SIZE>>>,
    protocol: Arc<P>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for CltSenderWithProtocol<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltSender] and calls [Protocol::on_send], [Protocol::on_wouldblock] and [Protocol::on_error] on respective events
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(&self.con_id, msg);
        let res = self.clt_sender.lock().send(msg); // release lock quickly
        match res {
            Ok(Completed) => {
                self.protocol.on_sent(&self.con_id, msg);
                Ok(Completed)
            }
            Ok(WouldBlock) => {
                self.protocol.on_wouldblock(&self.con_id, msg);
                Ok(WouldBlock)
            }
            Err(e) => {
                self.protocol.on_error(&self.con_id, msg, &e);
                Err(e)
            }
        }
    }
    /// Delegates to [CltSender] and calls [Protocol::on_send] and [Protocol::on_error] on respective events.
    /// Will only call [Protocol::on_wouldblock] once if timeout is reached
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <P as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(&self.con_id, msg);

        let start = Instant::now();
        loop {
            let res = self.clt_sender.lock().send(msg); // release lock quickly, don't lock using send_busywait_timeout
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(&self.con_id, msg);
                    return Ok(Completed);
                }
                Ok(WouldBlock) => {
                    if start.elapsed() > timeout {
                        self.protocol.on_wouldblock(&self.con_id, msg);
                        return Ok(WouldBlock);
                    }
                }
                Err(e) => {
                    self.protocol.on_error(&self.con_id, msg, &e);
                    return Err(e);
                }
            }
        }
    }
    /// Delegates to [CltSender] and calls [Protocol::on_send] and [Protocol::on_error] on respective events.
    /// Never calls [Protocol::on_wouldblock] and instead busywait until the message is sent
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(&self.con_id, msg);

        loop {
            let res = self.clt_sender.lock().send(msg); // release lock quickly, don't lock using send_busywait
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(&self.con_id, msg);
                    return Ok(());
                }
                Ok(WouldBlock) => continue,
                Err(e) => {
                    self.protocol.on_error(&self.con_id, msg, &e);
                    return Err(e);
                }
            }
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltSenderWithProtocol<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for CltSenderWithProtocol<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltSenderShared", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageRecver] and [MessageSender] that will issue respective [CallbackRecvSend] callback
/// while also supporting [Protocol] callbacks.
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
///         CltTestProtocolAuth::default(),
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
    protocol: Arc<P>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Clt<P, C, MAX_MSG_SIZE> {
    pub fn connect(addr: &str, timeout: Duration, retry_after: Duration, callback: Arc<C>, protocol: P, name: Option<&str>) -> Result<Self, Error> {
        assert!(timeout > retry_after, "timeout: {:?}, retry_after: {:?}", timeout, retry_after);
        let now = Instant::now();
        let con_id = ConId::clt(name, None, addr);
        while now.elapsed() < timeout {
            match TcpStream::connect(addr) {
                Err(e) => {
                    sleep(retry_after); // NOTE this will not be use by poll because it creates a client using a from_stream method
                    debug!("{} connection failed. e: {:?}", con_id, e);
                    continue;
                }
                Ok(stream) => {
                    let clt = Self::from_stream(stream, con_id, callback, protocol, None)?;
                    return Ok(clt);
                }
            }
        }
        let msg = format!("{:?} connect timeout: {:?}", con_id, timeout);
        Err(Error::new(std::io::ErrorKind::TimedOut, msg))
    }

    pub(crate) fn from_stream(stream: TcpStream, con_id: ConId, callback: Arc<C>, protocol: P, acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>) -> Result<Self, Error> {
        let (msg_recver, msg_sender) = into_split_messenger::<P, MAX_MSG_SIZE>(con_id, stream);
        let protocol = Arc::new(protocol);
        let mut con = Self {
            clt_recver: CltRecver::new(msg_recver, callback.clone(), acceptor_connection_gate.clone()),
            clt_sender: CltSender::new(msg_sender, callback.clone(), acceptor_connection_gate),
            protocol: protocol.clone(),
        };
        protocol.on_connected(&mut con)?;

        Ok(con)
    }
    pub fn into_split(self) -> (CltRecver<P, C, MAX_MSG_SIZE>, CltSender<P, C, MAX_MSG_SIZE>) {
        (self.clt_recver, self.clt_sender)
    }
    pub fn into_shared_split(self) -> (CltRecverWithProtocol<P, C, MAX_MSG_SIZE>, CltSenderWithProtocol<P, C, MAX_MSG_SIZE>) {
        let protocol = self.protocol.clone();
        let con_id = self.con_id().clone();
        let clt_sender = Arc::new(spin::Mutex::new(self.clt_sender));

        let clt_sender_shared_for_recv = CltSenderWithProtocol {
            con_id: con_id.clone(),
            clt_sender: clt_sender.clone(),
            protocol: protocol.clone(),
        };
        let clt_recver_with_shared_sender = CltRecverWithProtocol {
            clt_sender: clt_sender_shared_for_recv,
            clt_recver: self.clt_recver,
            protocol: protocol.clone(),
        };

        let clt_sender_shared_for_user = CltSenderWithProtocol { con_id, clt_sender, protocol };
        (clt_recver_with_shared_sender, clt_sender_shared_for_user)
    }
    pub fn into_spawned_sender(self) -> CltSender<P, C, MAX_MSG_SIZE> {
        let (recver, sender) = self.into_split();
        crate::connect::DEFAULT_POLL_HANDLER.add_recver(recver.into());
        sender
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltSender] and calls [Protocol::on_send], [Protocol::on_sent], [Protocol::on_wouldblock] and [Protocol::on_error] on respective events
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(self.con_id(), msg);

        match self.clt_sender.send(msg) {
            Ok(Completed) => {
                self.protocol.on_sent(self.con_id(), msg);
                Ok(Completed)
            }
            Ok(WouldBlock) => {
                self.protocol.on_wouldblock(self.con_id(), msg);
                Ok(WouldBlock)
            }
            Err(e) => {
                self.protocol.on_error(self.con_id(), msg, &e);
                Err(e)
            }
        }
    }
    /// Delegates to [CltSender] and calls [Protocol::on_send], [Protocol::on_sent] and [Protocol::on_error] on respective events.
    /// Will only call [Protocol::on_wouldblock] once if timeout is reached
    ///
    /// # Note
    /// This is an override implementation of [SendNonBlocking::send_busywait_timeout] that ensures that all protocol callbacks are called
    /// not more then once
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <P as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};
        let start = Instant::now();
        self.protocol.on_send(self.con_id(), msg);
        loop {
            let res = self.clt_sender.send(msg);
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(self.con_id(), msg);
                    return Ok(Completed);
                }
                Ok(WouldBlock) => {
                    self.protocol.on_wouldblock(self.con_id(), msg);
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    } else {
                        continue;
                    }
                }
                Err(e) => {
                    self.protocol.on_error(self.con_id(), msg, &e);
                    return Err(e);
                }
            }
        }
    }
    /// Delegates to [CltSender] and calls [Protocol::on_send], [Protocol::on_sent] and [Protocol::on_error] on respective events.
    /// Never calls [Protocol::on_wouldblock] and instead busywait until the message is sent
    ///
    /// # Note
    /// This is an override implementation of [SendNonBlocking::send_busywait] that ensures that all protocol callbacks are called
    /// not more then once
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(self.con_id(), msg);

        loop {
            let res = self.clt_sender.send(msg);
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(self.con_id(), msg);
                    return Ok(());
                }
                Ok(WouldBlock) => continue,
                Err(e) => {
                    self.protocol.on_error(self.con_id(), msg, &e);
                    return Err(e);
                }
            }
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::Completed;
        let res = self.clt_recver.recv();
        if let Ok(Completed(Some(ref msg))) = res {
            self.protocol.on_recv(msg, &mut self.clt_sender)?;
        }
        res
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for Clt<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.clt_recver.msg_recver.frm_reader.con_id
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
        let protocol = CltTestProtocolAuth::default();
        let res = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), callback, protocol, Some("unittest"));
        assert!(res.is_err());
    }
}
