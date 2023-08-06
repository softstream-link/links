use tokio::{sync::Mutex, task::AbortHandle};

use std::{
    any::type_name,
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::prelude::*;
use log::{debug, error};
use tokio::net::TcpStream;

use super::messenger::{into_split_messenger, MsgRecverRef, MsgSenderRef};

use tokio::{spawn, time::sleep};

#[derive(Debug)]
pub struct CltSender<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> {
    con_id: ConId,
    sender: MsgSenderRef<P, MMS>,
    callback: Arc<C>,
    protocol: Option<Arc<P>>,
    abort_handles: Vec<AbortHandle>,
    // callback: CallbackRef<M>, // TODO can't be fixed for now.
    // pub type CallbackRef<M> = Arc<Mutex<impl Callback<M>>>; // impl Trait` in type aliases is unstable see issue #63063 <https://github.com/rust-lang/rust/issues/63063>
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> CltSender<P, C, MMS> {
    pub async fn send(&self, msg: &mut P::SendT) -> Result<(), Box<dyn Error + Send + Sync>> {
        if let Some(protocol) = &self.protocol {
            protocol.on_send(&self.con_id, msg);
        }
        {
            self.callback.on_send(&self.con_id, msg);
        }
        {
            let mut writer = self.sender.lock().await;
            writer.send(msg).await
        }
    }
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> Display for CltSender<P, C, MMS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg_name = type_name::<P>().split("::").last().unwrap_or("Unknown");
        let clb_name = type_name::<C>()
            .split('<')
            .next()
            .unwrap_or("Unknown")
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{} CltSender<{}, {}, {}>",
            self.con_id, msg_name, clb_name, MMS
        )
    }
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> Drop for CltSender<P, C, MMS> {
    fn drop(&mut self) {
        for (idx, handle) in self.abort_handles.iter().enumerate() {
            debug!("{} {} change name aborting receiver", self, idx); // TODO change name of message
            handle.abort();
        }
        // self.recv_loop_abort_handle.abort();
    }
}

#[derive(Debug)]
pub struct Clt<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> {
    con_id: ConId,
    recver: MsgRecverRef<P, P>,
    sender: MsgSenderRef<P, MMS>,
    callback: Arc<C>,
    protocol: Option<Arc<P>>,
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> Clt<P, C, MMS> {
    pub async fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<C>,
        protocol: Option<Arc<P>>,
        name: Option<&str>,
        // TODO shall add custom Error type to be able to detect timeout?
    ) -> Result<CltSender<P, C, MMS>, Box<dyn Error + Send + Sync>> {
        assert!(timeout > retry_after);
        let now = Instant::now();
        let con_id = ConId::clt(name, None, addr);
        while now.elapsed() < timeout {
            let res = TcpStream::connect(addr).await;
            match res {
                Err(e) => {
                    debug!("{} connect failed. e: {:?}", con_id, e);
                    sleep(retry_after).await;
                    continue;
                }
                Ok(stream) => {
                    let mut con_id = con_id.clone();
                    con_id.set_local(stream.local_addr()?);
                    con_id.set_peer(stream.peer_addr()?);
                    let clt = Self::from_stream(stream, callback, protocol, con_id).await?;
                    debug!("{} connected", clt);
                    return Ok(clt);
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }

    pub(crate) async fn from_stream(
        stream: TcpStream,
        callback: Arc<C>,
        protocol: Option<Arc<P>>,
        con_id: ConId,
    ) -> Result<CltSender<P, C, MMS>, Box<dyn Error + Send + Sync>> {
        stream
            .set_nodelay(true)
            .expect("failed to set_nodelay=true");
        stream.set_linger(None).expect("failed to set_linger=None");
        let (sender, recver) = into_split_messenger::<P, MMS, P>(stream, con_id.clone());

        let recver = MsgRecverRef::new(Mutex::new(recver));
        let sender = MsgSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver,
            sender: Arc::clone(&sender),
            callback: Arc::clone(&callback),
            protocol: protocol.clone(),
        };

        // run protocol specific handshake sequence
        if let Some(ref protocol) = protocol {
            protocol.handshake(&clt).await?;
        }

        // start receiver loop
        let mut abort_handles = vec![spawn({
            let con_id = con_id.clone();
            async move {
                debug!("{} recv stream started", con_id);
                let res = Self::recv_loop(clt).await;
                match res {
                    Ok(()) => {
                        debug!("{} recv stream stopped", con_id);
                    }
                    Err(e) => {
                        error!("{} recv stream error: {:?}", con_id, e);
                        // TODO CRITICAL shall add panic?
                    }
                }
            }
        })
        .abort_handle()];

        // start protocol specific keep_alive loop
        if let Some(ref protocol) = protocol {
            abort_handles.push(
                spawn({
                    let con_id = con_id.clone();
                    let protocol = Arc::clone(&protocol);
                    let clt_sender = CltSender {
                        con_id: con_id.clone(),
                        sender: Arc::clone(&sender),
                        callback: Arc::clone(&callback),
                        protocol: Some(Arc::clone(&protocol)),
                        abort_handles: vec![],
                    };
                    async move {
                        debug!("{} keep_alive stream started", con_id);
                        let res = protocol.keep_alive_loop(clt_sender).await;
                        match res {
                            Ok(()) => {
                                debug!("{} keep_alive stream stopped", con_id);
                            }
                            Err(e) => {
                                error!("{} keep_alive stream error: {:?}", con_id, e);
                                // TODO CRITICAL shall add panic?
                            }
                        }
                    }
                })
                .abort_handle(),
            );
        }

        Ok(CltSender {
            con_id,
            sender,
            callback,
            protocol,
            abort_handles,
        })
    }

    async fn recv_loop(clt: Clt<P, C, MMS>) -> Result<(), Box<dyn Error + Sync + Send>> {
        let mut reader = clt.recver.lock().await;
        loop {
            // let opt_recv = clt.recv().await?; // Don't call clt.recv because it needs to re-acquire the lock on each call vs just holding it for the duration of the loop
            let opt = reader.recv().await?;
            match opt {
                Some(msg) => {
                    if let Some(ref protocol) = clt.protocol {
                        protocol.on_recv(&clt.con_id, &msg);
                    }
                    clt.callback.on_recv(&clt.con_id, msg)
                }
                None => break, // clean exist // end of stream
            }
        }
        Ok(())
    }

    pub async fn send(&self, msg: &P::SendT) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            self.callback.on_send(&self.con_id, msg);
        }
        {
            let mut writer = self.sender.lock().await;
            writer.send(msg).await
        }
    }
    pub async fn recv(&self) -> Result<Option<P::RecvT>, Box<dyn Error + Send + Sync>> {
        let res = {
            let mut reader = self.recver.lock().await;
            reader.recv().await
        };
        if let Ok(Some(ref msg)) = res {
            self.callback.on_recv(&self.con_id, msg.clone());
        }
        res
    }
    pub fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> Display for Clt<P, C, MMS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hdl_name = type_name::<P>().split("::").last().unwrap_or("Unknown");
        let clb_name = type_name::<C>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{} Clt<{}, {}, {}>",
            self.con_id, hdl_name, clb_name, MMS
        )
    }
}
impl<P: Protocol, C: CallbackSendRecv<P>, const MMS: usize> Drop for Clt<P, C, MMS> {
    fn drop(&mut self) {
        debug!("{} receiver stopped", self);
    }
}

#[cfg(test)]
mod test {

    use log::{info, Level};

    use super::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();

        let logger = LoggerCallback::new(Level::Debug).into();
        let clt = Clt::<_, _, 128>::connect(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            logger,
            Some(TestCltMsgProtocol.into()),
            None,
        )
        .await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
    // TODO move to soupbin
    // type SoupBinProtocol = SoupBinProtocolHandler<NoPayload>;
    // type SoupBinLoggerRef = LoggerCallbackRef<SoupBinProtocol>;
    // const MMS: usize = 128;
    // lazy_static! {
    //     static ref ADDR: String = setup::net::default_addr();
    //     static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
    //     static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    // }

    // #[tokio::test]
    // async fn test_clt_not_connected() {
    //     setup::log::configure();
    //     let logger: SoupBinLoggerRef = SoupBinLoggerRef::default();
    //     let clt = Clt::<SoupBinProtocol, _, MMS>::new(
    //         &ADDR,
    //         *CONNECT_TIMEOUT,
    //         *RETRY_AFTER,
    //         Arc::clone(&logger),
    //         None,
    //     )
    //     .await;

    //     info!("{:?}", clt);
    //     assert!(clt.is_err())
    // }
}
