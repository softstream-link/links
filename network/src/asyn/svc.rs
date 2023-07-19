use std::collections::VecDeque;
use std::fmt::Display;
use std::{error::Error, sync::Arc};

use framing::{Callback, ProtocolHandler};
use log::{error, info, warn};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::asyn::clt::Clt;
use framing::prelude::*;

use super::clt::CltSender;

// pub type SvcReaderRef<MESSENGER, FRAMER> = Arc<Mutex<Option<MessageRecver<MESSENGER, FRAMER>>>>;
#[rustfmt::skip]
pub type SvcSendersRef<MESSENGER, const MAX_MSG_SIZE: usize> = Arc<Mutex<VecDeque<CltSender<MESSENGER, MAX_MSG_SIZE>>>>;

// pub type CallbackRef<HANDLER> = Arc<Mutex<impl Callback<HANDLER>>>;

#[derive(Debug)]
pub struct SvcSender<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    senders: SvcSendersRef<MESSENGER, MAX_MSG_SIZE>,
    callback: Arc<Mutex<dyn Callback<MESSENGER>>>,
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> Display
    for SvcSender<MESSENGER, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> SvcSender<MESSENGER, MAX_MSG_SIZE> {
    pub async fn send(
        &mut self,
        msg: &MESSENGER::Message,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            let mut senders = self.senders.lock().await;
            for idx in 0..senders.len() {
                let mut sender = senders.pop_front().unwrap();
                match sender.send(msg).await {
                    Ok(_) => {
                        senders.push_back(sender);
                        drop(senders);

                        let callback = self.callback.lock().await;
                        callback.on_send(&self.con_id, msg);

                        return Ok(());
                    }
                    Err(err) => {
                        warn!(
                            "{} sender failure idx: {} evicting as disconnected err: {:?}",
                            sender, idx, err
                        );
                    }
                }
            }

            Err(format!("Not Connected senders len: {}", senders.len()).into()) // TODO better error type
        }
    }
}

#[derive(Debug)]
pub struct Svc<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> {
    // con_id: ConId,
    // // reader: SvcReaderRef<HANDLER>,
    // sender: SvcSendersRef<HANDLER, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<HANDLER>,
}

impl<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> Svc<HANDLER, MAX_MSG_SIZE> {
    pub async fn new(
        addr: &str,
        callback: impl Callback<HANDLER>,
    ) -> Result<SvcSender<HANDLER, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        let con_id = ConId::Svc(addr.to_owned());
        let lis = TcpListener::bind(&addr).await?;
        info!("{:?} bound successfully", con_id);

        let callback = Arc::new(Mutex::new(callback));
        let senders = SvcSendersRef::new(Mutex::new(VecDeque::new()));
        {
            let con_id = con_id.clone();
            let callback = Arc::clone(&callback);
            let senders = Arc::clone(&senders);
            tokio::spawn(async move {
                info!("{:?} accept loop started", con_id);
                match Self::service_recv(lis, callback, senders).await {
                    Ok(()) => info!("{:?} accept loop stopped", con_id),
                    Err(err) => error!("{:?} accept loop exit err: {:?}", con_id, err),
                }
            });
        }

        Ok(SvcSender {
            con_id: con_id.clone(),
            senders: Arc::clone(&senders),
            callback: callback.clone(),
        })
    }
    async fn service_recv(
        lis: TcpListener,
        callback: Arc<Mutex<impl Callback<HANDLER>>>,
        senders: SvcSendersRef<HANDLER, MAX_MSG_SIZE>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let con_id = ConId::Svc(format!(
                "{:?}<-{:?}",
                stream.local_addr()?,
                stream.peer_addr()?,
            ));

            let clt =
                Clt::<HANDLER, MAX_MSG_SIZE>::from_stream(stream, callback.clone(), con_id.clone())
                    .await;
            senders.lock().await.push_back(clt);
        }
    }
}

#[cfg(test)]
mod test {

    use soupbintcp4::prelude::*;

    use super::*;
    use crate::unittest::setup;
    use tokio::time::{sleep, Duration};
    type SoupBin = SoupBinMsg<NoPayload>;

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        type SoupBinNative = SoupBinProtocolHandler<NoPayload>;
        const MAX_MSG_SIZE: usize = 1024;
        let logger = LoggerCallback::<SoupBinNative>::new();

        let mut svc = Svc::<SoupBinNative, MAX_MSG_SIZE>::new(addr, logger)
            .await
            .unwrap();
        info!("{} sender ready", svc);
        loop {
            let x = svc.send(&SoupBin::dbg(b"hello from server")).await;
            info!("{} send result: {:?}", svc, x);
            sleep(Duration::from_secs(5)).await;
        }
    }
}
