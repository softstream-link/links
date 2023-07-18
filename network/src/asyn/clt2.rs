use tokio::sync::Mutex;

use std::{
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use framing::{MessageFramer, Messenger, Callback};
use log::info;
use tokio::net::TcpStream;

use super::con_msg2::{into_split_messenger, ConId, MessageRecver, MessageSender};

// use tokio::time::sleep;

// use tokio::spawn;

// use framing::MessageHandler;
// use tokio::net::TcpStream;

// use super::con_msg::{StreamMessenderReader, StreamMessenderWriter};
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

pub type CltMessageRecverRef<MESSENGER, FRAMER> = Arc<Mutex<MessageRecver<MESSENGER, FRAMER>>>;
pub type CltMessageSenderRef<MESSENGER, const MAX_MSG_SIZE: usize> =
    Arc<Mutex<MessageSender<MESSENGER, MAX_MSG_SIZE>>>;

#[derive(Debug)]
pub struct Clt<HANDLER: MessageFramer, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    recver: CltMessageRecverRef<HANDLER, HANDLER>,
    sender: CltMessageSenderRef<HANDLER, MAX_MSG_SIZE>,
}
impl<HANDLER: MessageFramer, const MAX_MSG_SIZE: usize> Display for Clt<HANDLER, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}

impl<HANDLER: MessageFramer, const MAX_MSG_SIZE: usize> Clt<HANDLER, MAX_MSG_SIZE> {
    pub async fn new(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
        callback: impl Callback,
    ) -> Result<CltSender<HANDLER, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        assert!(timeout > retry_after);
        let now = Instant::now();
        let con_id = ConId::Clt(addr.to_owned());
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
                    return Ok(Self::from_stream(stream, con_id).await);
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }

    pub async fn from_stream(stream: TcpStream, con_id: ConId) -> CltSender<HANDLER, MAX_MSG_SIZE> {
        let (sender, recver) =
            into_split_messenger::<HANDLER, MAX_MSG_SIZE, HANDLER>(stream, con_id.clone());

        let recv_ref = CltMessageRecverRef::new(Mutex::new(recver));
        let send_ref = CltMessageSenderRef::new(Mutex::new(sender));
        let clt = Self {
            con_id: con_id.clone(),
            recver: recv_ref.clone(),
            sender: send_ref.clone(),
        };
        {
            let con_id = con_id.clone();
            spawn(async move {
                info!("{:?} stream started", con_id);
                let res = Self::run(clt).await;
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
            sender: send_ref,
        }
    }

    async fn run(clt: Clt<HANDLER, MAX_MSG_SIZE>) -> Result<(), Box<dyn Error + Sync + Send>> {
        loop {
            let opt = {
                let mut clt_r_grd = clt.recver.lock().await;
                clt_r_grd.recv().await?
            };
            match opt {
                Some(msg) => {
                    info!("{:?} RECV: {:?}", clt.con_id, msg);
                    // TODO echo for now
                    // clt.sender.lock().await.send::<125>(&msg).await?; // TODO msg size
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
    use soupbintcp4::prelude::{NoPayload, SoupBinLoggerCallback, SoupBinMessageFramer};

    use super::*;
    use crate::unittest::setup;

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        let timeout = Duration::from_secs_f32(0.05);
        let logger = SoupBinLoggerCallback::<NoPayload>::default();
        type SoupBin = SoupBinMessageFramer<NoPayload>;
        const MAX_MSG_SIZE: usize = 1024;
        let clt = Clt::<SoupBin, MAX_MSG_SIZE>::new(addr, timeout, timeout / 5, logger).await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
    // #[tokio::test]
    // async fn test_clt() {
    //     setup::log::configure();
    //     let addr = &setup::net::default_addr();
    //     let timeout = Duration::from_secs(5);
    //     let mut clt = Clt::<SoupBinHandler<NoPayload>>::new(addr, timeout, timeout / 5)
    //         .await
    //         .unwrap();

    //     let msg = SoupBinMsg::dbg(b"hello world");
    //     clt.send::<1024>(&msg).await.unwrap();
    //     info!("{} sent msg: {:?}", clt, msg);

    //     sleep(Duration::from_secs(1)).await;
    // }
}
