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

use super::messaging::{into_split_messenger, MessageRecver, MessageSender};

use tokio::{spawn, time::sleep};

#[derive(Debug)]
pub struct CltSender<M, C, const MMS: usize>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    con_id: ConId,
    sender: CltSenderRef<M, MMS>,
    callback: Arc<C>,
    recver_abort_handle: AbortHandle,
    // callback: CallbackRef<M>, // TODO can't be fixed for now.
    // pub type CallbackRef<M> = Arc<Mutex<impl Callback<M>>>; // impl Trait` in type aliases is unstable see issue #63063 <https://github.com/rust-lang/rust/issues/63063>
}
impl<M, C, const MMS: usize> Display for CltSender<M, C, MMS>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg_name = type_name::<M>().split("::").last().unwrap_or("Unknown");
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
impl<M, C, const MMS: usize> Drop for CltSender<M, C, MMS>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    fn drop(&mut self) {
        debug!("{} aborting receiver", self);
        self.recver_abort_handle.abort();
    }
}

impl<M, C, const MMS: usize> CltSender<M, C, MMS>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    pub async fn send(&self, msg: &M::SendMsg) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            self.callback.on_send(&self.con_id, msg);
        }
        {
            let mut writer = self.sender.lock().await;
            writer.send(msg).await
        }
    }
}

pub use types::*;
#[rustfmt::skip]
mod types{
    use super::*;
    pub type CltRecverRef<M, FRAMER> = Arc<Mutex<MessageRecver<M, FRAMER>>>;
    pub type CltSenderRef<M, const MMS: usize> = Arc<Mutex<MessageSender<M, MMS>>>;
}

#[derive(Debug)]
pub struct Clt<P, C, const MMS: usize>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    con_id: ConId,
    recver: CltRecverRef<P, P>,
    sender: CltSenderRef<P, MMS>, // TODO possibly inject a protocol handler which will automatically reply and or send heartbeat for now keep the warning
    callback: Arc<C>,
}

impl<P, C, const MMS: usize> Display for Clt<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
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

impl<P, C, const MMS: usize> Drop for Clt<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    fn drop(&mut self) {
        debug!("{} receiver stopped", self);
    }
}
impl<P, C, const MMS: usize> Clt<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    pub async fn connect(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<C>,
        protocol: Option<P>,
        name: Option<&str>,
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
        protocol: Option<P>,
        con_id: ConId,
    ) -> Result<CltSender<P, C, MMS>, Box<dyn Error + Send + Sync>> {
        stream
            .set_nodelay(true)
            .expect("failed to set_nodelay=true");
        stream.set_linger(None).expect("failed to set_linger=None");
        let (sender, recver) = into_split_messenger::<P, MMS, P>(stream, con_id.clone());

        let recv_ref = CltRecverRef::new(Mutex::new(recver));
        let send_ref = CltSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver: Arc::clone(&recv_ref),
            sender: Arc::clone(&send_ref),
            callback: Arc::clone(&callback),
        };

        if let Some(ref protocol) = protocol {
            protocol.init_sequence(&clt).await?;
        }
        // TODO add protocol handler logic and exit logic

        let recver_abort_handle = spawn({
            let con_id = con_id.clone();
            async move {
                debug!("{} recv stream started", con_id);
                let res = Self::service_loop(clt, protocol).await;
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
        .abort_handle();

        Ok(CltSender {
            con_id,
            sender: Arc::clone(&send_ref),
            callback: Arc::clone(&callback),
            recver_abort_handle,
        })
    }

    async fn service_loop(
        clt: Clt<P, C, MMS>,
        _protocol: Option<P>,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let mut reader = clt.recver.lock().await;
        loop {
            // let opt_recv = clt.recv().await?; // Don't call clt.recv because it needs to re-acquire the lock
            let opt = reader.recv().await?;
            match opt {
                Some(msg) => clt.callback.on_recv(&clt.con_id, msg),
                None => break, // clean exist // end of stream
            }
        }
        Ok(())
    }

    pub async fn send(&self, msg: &P::SendMsg) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            self.callback.on_send(&self.con_id, msg);
        }
        {
            let mut writer = self.sender.lock().await;
            writer.send(msg).await
        }
    }
    pub async fn recv(&self) -> Result<Option<P::RecvMsg>, Box<dyn Error + Send + Sync>> {
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

#[cfg(test)]
mod test {

    use log::{info, Level};

    use super::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();

        let logger = LoggerCallback::new_ref(Level::Debug);
        let clt = Clt::<_, _, 128>::connect(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            logger,
            Some(CltMsgProtocol),
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
