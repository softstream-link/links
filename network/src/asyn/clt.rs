use tokio::sync::Mutex;

use std::{
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use framing::prelude::*;
use log::{debug, info};
use tokio::net::TcpStream;

use super::con_msg::{into_split_messenger, MessageRecver, MessageSender};

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
        write!(f, "{:?}", self.con_id)
    }
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> CltSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    pub async fn send(&self, msg: &MESSENGER::Message) -> Result<(), Box<dyn Error + Send + Sync>> {
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
    HANDLER: ProtocolHandler,
    CALLBACK: Callback<HANDLER>,
{
    con_id: ConId,
    recver: CltRecverRef<HANDLER, HANDLER>,
    sender: CltSenderRef<HANDLER, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<CALLBACK>,
}
impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Display for Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: ProtocolHandler,
    CALLBACK: Callback<HANDLER>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}

impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: ProtocolHandler,
    CALLBACK: Callback<HANDLER>,
{
    pub async fn new(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: Arc<CALLBACK>,
    ) -> Result<CltSender<HANDLER, CALLBACK, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        assert!(timeout > retry_after);
        let now = Instant::now();
        let con_id = ConId::Clt(addr.to_owned());
        while now.elapsed() < timeout {
            let res = TcpStream::connect(addr).await;
            match res {
                Err(e) => {
                    debug!("{:?} connect failed. e: {:?}", con_id, e);
                    sleep(retry_after).await;
                    continue;
                }
                Ok(stream) => {
                    let con_id = ConId::Clt(format!(
                        "{:?}->{:?}",
                        stream.local_addr()?,
                        stream.peer_addr()?
                    ));
                    debug!("{:?} connected", con_id);
                    return Ok(Self::from_stream(stream, callback, con_id).await);
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

        // TODO remove mutes on recv since it is only used in one place
        let recv_ref = CltRecverRef::new(Mutex::new(recver));
        let send_ref = CltSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver: Arc::clone(&recv_ref),
            sender: Arc::clone(&send_ref),
            phantom: std::marker::PhantomData,
        };

        spawn({
            let con_id = con_id.clone();
            let callback = Arc::clone(&callback);
            async move {
                debug!("{:?} stream started", con_id);
                let res = Self::service_recv(clt, con_id.clone(), callback).await;
                match res {
                    Ok(()) => {
                        debug!("{:?} stream stopped", con_id);
                    }
                    Err(e) => {
                        debug!("{:?} stream exit err:: {:?}", con_id, e);
                    }
                }
            }
        });

        CltSender {
            con_id: con_id.clone(),
            sender: Arc::clone(&send_ref),
            callback: Arc::clone(&callback),
        }
    }

    async fn service_recv(
        clt: Clt<HANDLER, CALLBACK, MAX_MSG_SIZE>,
        con_id: ConId,
        callback: Arc<CALLBACK>,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        loop {
            let opt = {
                let mut clt_grd = clt.recver.lock().await;
                clt_grd.recv().await?
            };
            match opt {
                Some(msg) => callback.on_recv(&con_id, msg),
                None => break, // clean exist
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use lazy_static::lazy_static;
    use soupbintcp4::prelude::*;

    use super::*;
    use crate::unittest::setup;

    type SoupBinProtocol = SoupBinProtocolHandler<NoPayload>;
    type SoupBinLoggerRef = LoggerCallbackRef<SoupBinProtocol>;
    const MAX_MSG_SIZE: usize = 128;
    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();
        let logger: SoupBinLoggerRef = SoupBinLoggerRef::default();
        let clt = Clt::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            Arc::clone(&logger),
        )
        .await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
}
