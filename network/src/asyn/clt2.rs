use tokio::sync::Mutex;

use std::{
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use framing::{Callback, Messenger, ProtocolHandler};
use log::info;
use tokio::net::TcpStream;

use super::con_msg2::{into_split_messenger, ConId, MessageRecver, MessageSender};

use tokio::{spawn, time::sleep};

#[derive(Debug)]
pub struct CltSender<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    sender: CltMessageSenderRef<MESSENGER, MAX_MSG_SIZE>,
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> Display
    for CltSender<MESSENGER, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> CltSender<MESSENGER, MAX_MSG_SIZE> {
    pub async fn send(
        &mut self,
        msg: &mut MESSENGER::Message,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut writer = self.sender.lock().await;
        writer.send(msg).await
    }
}

pub use types::*;
#[rustfmt::skip]
mod types{
    use super::*;
    pub type CltMessageRecverRef<MESSENGER, FRAMER> = Arc<Mutex<MessageRecver<MESSENGER, FRAMER>>>;
    pub type CltMessageSenderRef<MESSENGER, const MAX_MSG_SIZE: usize> = Arc<Mutex<MessageSender<MESSENGER, MAX_MSG_SIZE>>>;
}

#[derive(Debug)]
pub struct Clt<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    recver: CltMessageRecverRef<HANDLER, HANDLER>,
    _sender: CltMessageSenderRef<HANDLER, MAX_MSG_SIZE>,
}
impl<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> Display for Clt<HANDLER, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}

impl<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> Clt<HANDLER, MAX_MSG_SIZE> {
    pub async fn new(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: impl Callback<HANDLER>,
    ) -> Result<CltSender<HANDLER, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        assert!(timeout > retry_after);
        let now = Instant::now();
        let con_id = ConId::Clt(addr.to_owned());
        let callback = Arc::new(Mutex::new(callback));
        while now.elapsed() < timeout {
            let res = TcpStream::connect(addr).await;
            match res {
                Err(e) => {
                    info!("{:?} connect failed. e: {:?}", con_id, e);
                    sleep(retry_after).await;
                    continue;
                }
                Ok(stream) => {
                    let con_id = ConId::Clt(format!(
                        "{:?}->{:?}",
                        stream.local_addr()?,
                        stream.peer_addr()?
                    ));
                    info!("{:?} connected", con_id);
                    return Ok(Self::from_stream(stream, callback, con_id).await);
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }

    pub async fn from_stream(
        stream: TcpStream,
        // callback: impl Callback<HANDLER>,
        callback: Arc<Mutex<impl Callback<HANDLER>>>,
        con_id: ConId,
    ) -> CltSender<HANDLER, MAX_MSG_SIZE> {
        let (sender, recver) =
            into_split_messenger::<HANDLER, MAX_MSG_SIZE, HANDLER>(stream, con_id.clone());

        let recv_ref = CltMessageRecverRef::new(Mutex::new(recver));
        let send_ref = CltMessageSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver: Arc::clone(&recv_ref),
            _sender: Arc::clone(&send_ref),
        };
        {
            let con_id = con_id.clone();
            spawn(async move {
                info!("{:?} stream started", con_id);
                let res = Self::run(clt, callback).await;
                match res {
                    Ok(()) => {
                        info!("{:?} stream stopped", con_id);
                    }
                    Err(e) => {
                        info!("{:?} stream exit err:: {:?}", con_id, e);
                    }
                }
            });
        }

        CltSender {
            con_id: con_id.clone(),
            sender: Arc::clone(&send_ref),
        }
    }

    async fn run(
        clt: Clt<HANDLER, MAX_MSG_SIZE>,
        callback: Arc<Mutex<impl Callback<HANDLER>>>,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let mut callback = callback.lock().await;
        loop {
            let opt = {
                let mut clt_r_grd = clt.recver.lock().await;
                clt_r_grd.recv().await?
            };
            match opt {
                Some(msg) => {
                    callback.on_recv(msg);
                }
                None => {
                    return Ok(()); // clean exist
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use framing::LoggerCallback;
    use soupbintcp4::prelude::{NoPayload, SoupBinMsg, SoupBinProtocolHandler};

    use super::*;
    use crate::unittest::setup;

    type SoupBin = SoupBinMsg<NoPayload>;
    type SoupBinNative = SoupBinProtocolHandler<NoPayload>;
    const MAX_MSG_SIZE: usize = 1024;

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        let timeout = Duration::from_secs_f32(0.05);
        let logger = LoggerCallback::<SoupBinNative>::new();
        let clt = Clt::<SoupBinNative, MAX_MSG_SIZE>::new(addr, timeout, timeout / 5, logger).await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        let timeout = Duration::from_secs(5);
        let logger = LoggerCallback::<SoupBinNative>::new();
        let mut clt = Clt::<SoupBinNative, MAX_MSG_SIZE>::new(addr, timeout, timeout / 5, logger)
            .await
            .unwrap();

        let msg = &mut SoupBin::dbg(b"hello world");
        clt.send(msg).await.unwrap();
        // info!("{} sent msg: {:?}", clt, msg);

        sleep(Duration::from_secs(1)).await;
    }
}
