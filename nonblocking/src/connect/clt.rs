use crate::prelude::{into_split_messenger, CallbackRecv, CallbackRecvSend, CallbackSend, ConId, MessageRecver, MessageSender, Messenger, PollEventStatus, PollReadable, Protocol, RecvNonBlocking, RecvStatus, SendNonBlocking, SendNonBlockingNonMut, SendStatus};
use links_core::{
    asserted_short_name,
    core::{conid::ConnectionId, counters::max_connection::RemoveConnectionBarrierOnDrop},
};
use log::debug;
use std::{
    fmt::{Debug, Display},
    io::Error,
    net::TcpStream,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

/// An abstraction over a [MessageRecver] that executes [Protocol::on_recv] and [CallbackRecv::on_recv] callbacks on every message being processed by [CltRecver].
/// It is designed to work in a single thread that is different from [CltSender] thread.
///
/// # Important
/// This is an owned implementation and is not [Clone]able or [Sync]able.
#[derive(Debug)]
pub struct CltRecver<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> {
    msg_recver: MessageRecver<P, MAX_MSG_SIZE>,
    callback: Arc<C>,
    protocol: Arc<P>,
    #[allow(dead_code)] // exists to indicate to Svc::accept that this connection no longer active when Self is dropped
    acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>,
}
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> CltRecver<P, C, MAX_MSG_SIZE> {
    pub fn new(recver: MessageRecver<P, MAX_MSG_SIZE>, callback: Arc<C>, protocol: Arc<P>, acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>) -> Self {
        Self {
            msg_recver: recver,
            callback,
            protocol,
            acceptor_connection_gate,
        }
    }
}
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for CltRecver<P, C, MAX_MSG_SIZE> {
    // NOTE: that the [RecvNonBlocking::recv_busywait] & [RecvNonBlocking::recv_busywait_timeout] default implementation
    // is not overridden because the callback is only issues when [RecvStatus::Completed] is returned, hence default implementation is sufficient
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<P::RecvT>, Error> {
        match self.msg_recver.recv()? {
            RecvStatus::Completed(Some(msg)) => {
                self.protocol.on_recv(self, &msg);
                self.callback.on_recv(self.con_id(), &msg);
                Ok(RecvStatus::Completed(Some(msg)))
            }
            RecvStatus::Completed(None) => Ok(RecvStatus::Completed(None)),
            RecvStatus::WouldBlock => Ok(RecvStatus::WouldBlock),
        }
    }
}
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltRecver<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.msg_recver.frm_reader.con_id
    }
}
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> PollReadable for CltRecver<P, C, MAX_MSG_SIZE> {
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
impl<P: Protocol, C: CallbackRecv<P>, const MAX_MSG_SIZE: usize> Display for CltRecver<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltRecver", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageSender] that executes [Protocol::on_send], [Protocol::on_sent]/[Protocol::on_wouldblock]/[Protocol::on_error] and [CallbackSend::on_sent] on every message processed by [CltSender].
/// It is designed to work in a single thread that is different from [CltRecver] thread
///
/// # Important
/// This is an owned implementation and is not [Clone]able.
#[derive(Debug)]
pub struct CltSender<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> {
    msg_sender: MessageSender<P, MAX_MSG_SIZE>,
    callback: Arc<C>,
    protocol: Arc<P>,
    #[allow(dead_code)] // exists to indicate to Svc::accept that this connection no longer active when Self is dropped
    acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>,
}
impl<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> CltSender<P, C, MAX_MSG_SIZE> {
    pub fn new(sender: MessageSender<P, MAX_MSG_SIZE>, callback: Arc<C>, protocol: Arc<P>, acceptor_connection_gate: Option<RemoveConnectionBarrierOnDrop>) -> Self {
        Self {
            msg_sender: sender,
            callback,
            protocol,
            acceptor_connection_gate,
        }
    }
}
impl<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for CltSender<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.protocol.on_send(self, msg);
        let res = self.msg_sender.send(msg);
        match res {
            Ok(SendStatus::Completed) => {
                self.protocol.on_sent(self, msg);
                self.callback.on_sent(self.con_id(), msg);
                Ok(SendStatus::Completed)
            }
            Ok(SendStatus::WouldBlock) => {
                self.protocol.on_wouldblock(self, msg);
                Ok(SendStatus::WouldBlock)
            }
            Err(e) => {
                self.protocol.on_error(self, msg, &e);
                Err(e)
            }
        }
    }

    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <P as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        // NOTE: that the [SendNonBlocking::send_busywait_timeout] default implementation is overridden to ensure correct callback sequence
        use SendStatus::{Completed, WouldBlock};
        let start = Instant::now();
        self.protocol.on_send(self, msg);
        loop {
            let res = self.msg_sender.send(msg);
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(self, msg);
                    self.callback.on_sent(self.con_id(), msg);
                    return Ok(Completed);
                }
                Ok(WouldBlock) => {
                    if start.elapsed() > timeout {
                        self.protocol.on_wouldblock(self, msg);
                        return Ok(WouldBlock);
                    } else {
                        continue;
                    }
                }
                Err(e) => {
                    self.protocol.on_error(self, msg, &e);
                    return Err(e);
                }
            }
        }
    }
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<(), Error> {
        // NOTE: that the [SendNonBlocking::send_busywait] default implementation is overridden to ensure correct callback sequence
        use SendStatus::{Completed, WouldBlock};
        self.protocol.on_send(self, msg);
        loop {
            let res = self.msg_sender.send(msg);
            match res {
                Ok(Completed) => {
                    self.protocol.on_sent(self, msg);
                    self.callback.on_sent(self.con_id(), msg);
                    return Ok(());
                }
                Ok(WouldBlock) => continue,
                Err(e) => {
                    self.protocol.on_error(self, msg, &e);
                    return Err(e);
                }
            }
        }
    }
}
impl<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltSender<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.msg_sender.frm_writer.con_id
    }
}
impl<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> Display for CltSender<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltSender", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// A reference counted abstraction which delegates all calls to [CltRecver] protected by a [spin::Mutex]
/// It is designed to cloned and shared across threads at the cost of spin lock on every call.
///
/// # Important
/// In addition to call delegating it enables enhanced features of the [Protocol] trait, such as [Protocol::do_reply]
/// by holding a reference to clone of [CltSenderRef]
#[derive(Debug, Clone)]
pub struct CltRecverRef<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> {
    con_id: ConId, // this is a clone copy fro CltSender to avoid mutex call to id a connection
    clt_recver: Arc<spin::Mutex<CltRecver<P, C, MAX_MSG_SIZE>>>,
    clt_sender: CltSenderRef<P, C, MAX_MSG_SIZE>,
    protocol: Arc<P>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for CltRecverRef<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltRecver] and calls [Protocol::do_reply] when a message is received
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::Completed;
        let res = self.clt_recver.lock().recv();
        if let Ok(Completed(Some(ref msg))) = res {
            self.protocol.do_reply(msg, &mut self.clt_sender)?;
        }
        res
    }
    /// Delegates to [CltRecver] and calls [Protocol::do_reply] when a message is received
    #[inline(always)]
    fn recv_busywait_timeout(&mut self, timeout: Duration) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        // NOTE: that the [RecvNonBlocking::recv_busywait_timeout] default implementation is overridden to reduce lock contention
        use RecvStatus::{Completed, WouldBlock};
        let start = Instant::now();
        loop {
            let status = self.clt_recver.lock().recv()?;
            match status {
                Completed(Some(msg)) => {
                    self.protocol.do_reply(&msg, &mut self.clt_sender)?;
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
    /// Delegates to [CltRecver] and calls [Protocol::on_reply] when the message is received.
    #[inline(always)]
    fn recv_busywait(&mut self) -> Result<Option<<P as Messenger>::RecvT>, Error> {
        // NOTE: that the [RecvNonBlocking::recv_busywait] default implementation is overridden to reduce lock contention
        use RecvStatus::{Completed, WouldBlock};
        loop {
            let status = self.clt_recver.lock().recv()?; // release lock quickly, don't lock using recv_busywait
            match status {
                Completed(Some(msg)) => {
                    self.protocol.do_reply(&msg, &mut self.clt_sender)?;
                    return Ok(Some(msg));
                }
                Completed(None) => return Ok(None),
                WouldBlock => continue,
            }
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltRecverRef<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> PollReadable for CltRecverRef<P, C, MAX_MSG_SIZE> {
    fn register(&mut self, registry: &mio::Registry, token: mio::Token, interests: mio::Interest) -> Result<(), Error> {
        let mut guard = self.clt_recver.lock();
        registry.register(&mut guard.msg_recver.frm_reader.stream_reader, token, interests)
    }
    fn deregister(&mut self, registry: &mio::Registry) -> Result<(), Error> {
        let mut guard = self.clt_recver.lock();
        registry.deregister(&mut guard.msg_recver.frm_reader.stream_reader)
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        panic!("Invalid API usage. PollReadable::register and PollReadable::deregister are overridden and this call shall never be issued.")
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
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for CltRecverRef<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltRecverWithSenderRef", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// A reference counted abstraction which delegates all calls to [CltSender] protected by a [spin::Mutex]
/// It is designed to cloned and shared across threads at the cost of spin lock on every call.s
#[derive(Debug, Clone)]
pub struct CltSenderRef<P: Protocol, C: CallbackSend<P>, const MAX_MSG_SIZE: usize> {
    con_id: ConId, // this is a clone copy fro CltSender to avoid mutex call to id a connection
    clt_sender: Arc<spin::Mutex<CltSender<P, C, MAX_MSG_SIZE>>>,
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for CltSenderRef<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltSender] once a spin lock is acquired.
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clt_sender.lock().send(msg)
    }
    /// Delegates to [CltSender] once a spin lock is acquired.
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <P as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        use SendStatus::{Completed, WouldBlock};

        let start = Instant::now();
        loop {
            let status = self.clt_sender.lock().send(msg)?; // release lock quickly, don't lock using send_busywait_timeout
            match status {
                Completed => return Ok(Completed),
                WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WouldBlock);
                    }
                }
            }
        }
    }
    /// Delegates to [CltSender] once a spin lock is acquired.
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<(), Error> {
        use SendStatus::{Completed, WouldBlock};
        loop {
            let status = self.clt_sender.lock().send(msg)?; // release lock quickly, don't lock using send_busywait
            match status {
                Completed => return Ok(()),
                WouldBlock => continue,
            }
        }
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> ConnectionId for CltSenderRef<P, C, MAX_MSG_SIZE> {
    #[inline(always)]
    fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> Display for CltSenderRef<P, C, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let recv_t = std::any::type_name::<P::RecvT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        let send_t = std::any::type_name::<P::SendT>().split("::").last().unwrap_or("Unknown").replace('>', "");
        write!(f, "{}<{}, RecvT:{}, SendT:{}, {}>", asserted_short_name!("CltSenderShared", Self), self.con_id(), recv_t, send_t, MAX_MSG_SIZE)
    }
}

/// An abstraction over a [MessageRecver] and [MessageSender] that executes [Protocol] and [CallbackRecvSend] callbacks on every message being processed by [CltRecver] and [CltSender] respectively.
/// It is designed to work in a single thread. To split use:
/// * [Clt::into_split] - for [CltRecver]/[CltSender]
/// * [Clt::into_split_ref] - for [CltRecverRef]/[CltSenderRef]
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
            clt_recver: CltRecver::new(msg_recver, callback.clone(), protocol.clone(), acceptor_connection_gate.clone()),
            clt_sender: CltSender::new(msg_sender, callback.clone(), protocol.clone(), acceptor_connection_gate),
            protocol: protocol.clone(),
        };
        protocol.on_connected(&mut con)?;

        Ok(con)
    }
    pub fn into_spawned_sender(self) -> impl SendNonBlocking<P> {
        let (recver, sender) = self.into_split();
        crate::connect::DEFAULT_POLL_HANDLER.add_recver(recver.into());
        sender
    }
    pub fn into_spawned_ref(self) -> impl SendNonBlocking<P> {
        let (recver, sender) = self.into_split_ref();
        crate::connect::DEFAULT_POLL_HANDLER.add_recver(recver.into());
        sender
    }
    pub fn into_split(self) -> (CltRecver<P, C, MAX_MSG_SIZE>, CltSender<P, C, MAX_MSG_SIZE>) {
        (self.clt_recver, self.clt_sender)
    }
    pub fn into_split_ref(self) -> (CltRecverRef<P, C, MAX_MSG_SIZE>, CltSenderRef<P, C, MAX_MSG_SIZE>) {
        let protocol = self.protocol.clone();
        let con_id = self.con_id().clone();
        let clt_sender = Arc::new(spin::Mutex::new(self.clt_sender));

        let clt_sender_shared_for_recv = CltSenderRef { con_id: con_id.clone(), clt_sender: clt_sender.clone() };
        let clt_recver_with_shared_sender = CltRecverRef {
            con_id: con_id.clone(),
            clt_recver: Arc::new(spin::Mutex::new(self.clt_recver)),
            clt_sender: clt_sender_shared_for_recv,
            protocol: protocol.clone(),
        };

        let clt_sender_shared_for_user = CltSenderRef { con_id, clt_sender };
        (clt_recver_with_shared_sender, clt_sender_shared_for_user)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> SendNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltSender]
    #[inline(always)]
    fn send(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<SendStatus, Error> {
        self.clt_sender.send(msg)
    }
    /// Delegates to [CltSender]
    #[inline(always)]
    fn send_busywait_timeout(&mut self, msg: &mut <P as Messenger>::SendT, timeout: Duration) -> Result<SendStatus, Error> {
        self.clt_sender.send_busywait_timeout(msg, timeout)
    }
    /// Delegates to [CltSender]
    #[inline(always)]
    fn send_busywait(&mut self, msg: &mut <P as Messenger>::SendT) -> Result<(), Error> {
        self.clt_sender.send_busywait(msg)
    }
}
impl<P: Protocol, C: CallbackRecvSend<P>, const MAX_MSG_SIZE: usize> RecvNonBlocking<P> for Clt<P, C, MAX_MSG_SIZE> {
    /// Delegates to [CltRecver]
    #[inline(always)]
    fn recv(&mut self) -> Result<RecvStatus<<P as Messenger>::RecvT>, Error> {
        use RecvStatus::Completed;
        let res = self.clt_recver.recv();
        if let Ok(Completed(Some(ref msg))) = res {
            self.protocol.do_reply(msg, &mut self.clt_sender)?;
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
