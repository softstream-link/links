use std::collections::VecDeque;
use std::fmt::Display;
use std::{error::Error, sync::Arc};

use framing::prelude::*;
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex};

use crate::asyn::clt::Clt;

use super::clt::CltSender;

// pub type SvcReaderRef<MESSENGER, FRAMER> = Arc<Mutex<Option<MessageRecver<MESSENGER, FRAMER>>>>;
#[rustfmt::skip]
pub type SvcSendersRef<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> = Arc<Mutex<VecDeque<CltSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>>>>;

// pub type CallbackRef<HANDLER> = Arc<Mutex<impl Callback<HANDLER>>>;

#[derive(Debug)]
pub struct SvcSender<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    con_id: ConId,
    senders: SvcSendersRef<MESSENGER, CALLBACK, MAX_MSG_SIZE>,
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> Display
    for SvcSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> SvcSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    pub async fn is_accepted(&self) -> bool {
        let senders = self.senders.lock().await;
        !senders.is_empty()
    }
    pub async fn send(&self, msg: &MESSENGER::Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        {
            let mut senders = self.senders.lock().await;
            for idx in 0..senders.len() {
                let sender = senders
                    .pop_front()
                    .expect("senders can't be empty since we are in the loop");
                match sender.send(msg).await {
                    Ok(_) => {
                        senders.push_back(sender);
                        drop(senders);
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
pub struct Svc<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize>
where
    HANDLER: ProtocolHandler,
    CALLBACK: Callback<HANDLER>,
{
    p1: std::marker::PhantomData<HANDLER>,
    p2: std::marker::PhantomData<CALLBACK>,
}

impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Svc<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: ProtocolHandler,
    CALLBACK: Callback<HANDLER>,
{
    pub async fn new(
        addr: &str,
        callback: Arc<CALLBACK>,
        name: Option<&str>,
    ) -> Result<SvcSender<HANDLER, CALLBACK, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        let con_id = ConId::svc(name, addr, None);
        let lis = TcpListener::bind(&addr).await?;
        debug!("{:?} bound successfully", con_id);

        let senders = SvcSendersRef::new(Mutex::new(VecDeque::new()));

        tokio::spawn({
            let con_id = con_id.clone();
            let callback = Arc::clone(&callback);
            let senders = Arc::clone(&senders);
            async move {
                debug!("{:?} accept loop started", con_id);
                match Self::service_accept(lis, callback, senders, con_id.clone()).await {
                    Ok(()) => debug!("{:?} accept loop stopped", con_id),
                    Err(err) => error!("{:?} accept loop exit err: {:?}", con_id, err),
                }
            }
        });

        Ok(SvcSender {
            con_id: con_id.clone(),
            senders: Arc::clone(&senders),
        })
    }
    async fn service_accept(
        lis: TcpListener,
        callback: Arc<CALLBACK>,
        senders: SvcSendersRef<HANDLER, CALLBACK, MAX_MSG_SIZE>,
        con_id: ConId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let mut con_id = con_id.clone();
            con_id.set_local(stream.local_addr()?);
            con_id.set_peer(stream.peer_addr()?);

            let clt = Clt::<HANDLER, CALLBACK, MAX_MSG_SIZE>::from_stream(
                stream,
                callback.clone(),
                con_id.clone(),
            )
            .await;
            senders.lock().await.push_back(clt);
        }
    }
}

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use log::info;
    use soupbintcp4::prelude::*;

    use super::*;
    use crate::unittest::setup;
    use tokio::{
        spawn,
        time::{sleep, Duration},
    };

    type SoupBin = SoupBinMsg<NoPayload>;
    type SoupBinProtocol = SoupBinProtocolHandler<NoPayload>;
    type SoupBinChainRef = ChainCallbackRef<SoupBinProtocol>;
    type SoupBinLoggerRef = LoggerCallbackRef<SoupBinProtocol>;
    type SoupBinEvenLogRef = EventLogCallbackRef<SoupBinProtocol>;

    const MAX_MSG_SIZE: usize = 128;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
        static ref FIND_TIMEOUT: Duration = setup::net::default_find_timeout();
    }

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        let logger = SoupBinLoggerRef::default();
        let svc = Svc::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(&ADDR, Arc::clone(&logger), None)
            .await
            .unwrap();
        info!("{} sender ready", svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure();
        let find_timeout = setup::net::default_find_timeout();
        let event_log = SoupBinEvenLogRef::default();
        let callback = SoupBinChainRef::new(ChainCallback::new(vec![
            SoupBinLoggerRef::default(),
            event_log.clone(),
        ]));

        let svc = Svc::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(&ADDR, Arc::clone(&callback), None)
            .await
            .unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            Arc::clone(&callback),
            None,
        )
        .await
        .unwrap();
        info!("{} sender ready", clt);

        while !svc.is_accepted().await {}

        let msg_clt = SoupBin::dbg(b"hello from client");
        let msg_svc = SoupBin::dbg(b"hello from server");
        clt.send(&msg_clt).await.unwrap();
        svc.send(&msg_svc).await.unwrap();

        let found = event_log
            .find(
                |entry| entry.direction == Direction::Recv && entry.msg == msg_svc,
                // TODO need a name
                find_timeout.into(),
            )
            .await;
        info!("event_log: {}", *event_log);
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert_eq!(found.unwrap().msg, msg_svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connection1() {
        setup::log::configure();
        // let find_timeout = setup::net::default_find_timeout();
        // let event_log = SoupBinEvenLogRef::default();
        let callback = SoupBinLoggerRef::default();
        // let callback = SoupBinChainRef::new(ChainCallback::new(vec![
        //     SoupBinLoggerRef::default(),
        //     event_log.clone(),
        // ]));

        let svc = Svc::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(&ADDR, Arc::clone(&callback), None)
            .await
            .unwrap();

        info!("{} sender ready", svc);

 
        for _ in 0..2 {
            let clt = Clt::<SoupBinProtocol, _, MAX_MSG_SIZE>::new(
                &ADDR,
                *CONNECT_TIMEOUT,
                *RETRY_AFTER,
                Arc::clone(&callback),
                None,
            )
            .await
            .unwrap();

            // info!("{} sender ready", clt);
            
            drop(clt);
            sleep(*CONNECT_TIMEOUT).await;
        }

        spawn(async move {
            loop {
                if !svc.is_accepted().await {
                    continue;
                }
                let msg_svc = SoupBin::dbg(b"hello from server");
                let res = svc.send(&msg_svc).await;
                info!("svc send res: {:?}", res);
                break;
            }
        });
        sleep(*CONNECT_TIMEOUT).await;
    }
}
