use tokio::{sync::Mutex, task::AbortHandle};

use std::{
    any::type_name,
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::prelude::*;
use log::{debug, error, warn};
use tokio::net::TcpStream;

use super::messaging::{into_split_messenger, MessageRecver, MessageSender};

use tokio::{spawn, time::sleep};

#[derive(Debug)]
pub struct CltSender<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    con_id: ConId,
    sender: CltSenderRef<MESSENGER, MAX_MSG_SIZE>,
    callback: Arc<CALLBACK>,
    recver_abort_handle: AbortHandle,
    // callback: CallbackRef<MESSENGER>, // TODO can't be fixed for now.
    // pub type CallbackRef<MESSENGER> = Arc<Mutex<impl Callback<MESSENGER>>>; // impl Trait` in type aliases is unstable see issue #63063 <https://github.com/rust-lang/rust/issues/63063>
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> Display
    for CltSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg_name = type_name::<MESSENGER>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        let clb_name = type_name::<CALLBACK>()
            .split("<")
            .next()
            .unwrap_or("Unknown")
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{} CltSender<{}, {}, {}>",
            self.con_id, msg_name, clb_name, MAX_MSG_SIZE
        )
    }
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> Drop
    for CltSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    fn drop(&mut self) {
        debug!("{} aborting receiver", self);
        self.recver_abort_handle.abort();
    }
}

impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> CltSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    pub async fn send(&self, msg: &MESSENGER::SendMsg) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            // let callback = self.callback.lock().await;
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
    pub type CltRecverRef<MESSENGER, FRAMER> = Arc<Mutex<MessageRecver<MESSENGER, FRAMER>>>;
    pub type CltSenderRef<MESSENGER, const MAX_MSG_SIZE: usize> = Arc<Mutex<MessageSender<MESSENGER, MAX_MSG_SIZE>>>;
}

#[derive(Debug)]
pub struct Clt<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize>
where
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    con_id: ConId,
    recver: CltRecverRef<HANDLER, HANDLER>,
    sender: CltSenderRef<HANDLER, MAX_MSG_SIZE>,
    callback: Arc<CALLBACK>,
}

impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Display for Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hdl_name = type_name::<HANDLER>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        let clb_name = type_name::<CALLBACK>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{} Clt<{}, {}, {}>",
            self.con_id, hdl_name, clb_name, MAX_MSG_SIZE
        )
    }
}
impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Drop for Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    fn drop(&mut self) {
        debug!("{} receiver stopped", self);
    }
}
impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    pub async fn new(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<CALLBACK>,
        name: Option<&str>,
    ) -> Result<CltSender<HANDLER, CALLBACK, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
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
                    let clt = Self::from_stream(stream, callback, con_id).await;
                    debug!("{} connected", clt);
                    return Ok(clt);
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }

    pub async fn from_stream(
        stream: TcpStream,
        callback: Arc<CALLBACK>,
        con_id: ConId,
    ) -> CltSender<HANDLER, CALLBACK, MAX_MSG_SIZE> {
        let (sender, recver) =
            into_split_messenger::<HANDLER, MAX_MSG_SIZE, HANDLER>(stream, con_id.clone());

        let recv_ref = CltRecverRef::new(Mutex::new(recver));
        let send_ref = CltSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver: Arc::clone(&recv_ref),
            sender: Arc::clone(&send_ref),
            callback: Arc::clone(&callback),
        };

        let recver_abort_handle = spawn({
            let con_id = con_id.clone();
            async move {
                debug!("{} recv stream started", con_id);
                let res = Self::service_recv(clt, con_id.clone()).await;
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

        CltSender {
            con_id: con_id.clone(),
            sender: Arc::clone(&send_ref),
            callback: Arc::clone(&callback),
            recver_abort_handle,
        }
    }

    async fn service_recv(
        clt: Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>,
        con_id: ConId,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        loop {
            let opt = {
                let mut clt_grd = clt.recver.lock().await;
                clt_grd.recv().await?
                // TODO add protocol handler logic and exit logic
            };
            match opt {
                Some(msg) => clt.callback.on_recv(&con_id, msg),
                None => break, // clean exist
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use super::*;
    use crate::unittest::setup::{self, protocol::*};

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();
        const MAX_MSG_SIZE: usize = 128;
        let logger = LoggerCallbackRef::<CltMsgProtocol>::default();
        // TODO remove MsgProtocolHandler type parameter once implmentedn as instance and passed as argument
        let clt = Clt::<_, _, MAX_MSG_SIZE>::new(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            Arc::clone(&logger),
            None,
        )
        .await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
    // TODO move to soupbin
    // type SoupBinProtocol = SoupBinProtocolHandler<NoPayload>;
    // type SoupBinLoggerRef = LoggerCallbackRef<SoupBinProtocol>;
    // const MAX_MSG_SIZE: usize = 128;
    // lazy_static! {
    //     static ref ADDR: String = setup::net::default_addr();
    //     static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
    //     static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    // }

    // #[tokio::test]
    // async fn test_clt_not_connected() {
    //     setup::log::configure();
    //     let logger: SoupBinLoggerRef = SoupBinLoggerRef::default();
    //     let clt = Clt::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(
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
