use std::{any::type_name, collections::VecDeque, error::Error, fmt::Display, sync::Arc};

use crate::prelude::*;
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex, task::AbortHandle};

use super::clt::{Clt, CltSender};

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
    recver_abort_handle: AbortHandle,
}
impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> Display
    for SvcSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
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
        let con_name = match &self.con_id {
            ConId::Svc { name, local, .. } => format!("Svc({}@{})", name, local),
            _ => panic!("SvcSender has Invalid ConId: {:?}", self.con_id),
        };
        write!(
            f,
            "{} SvcSender<{}, {}, {}>",
            con_name, msg_name, clb_name, MAX_MSG_SIZE
        )
    }
}

impl<MESSENGER, CALLBACK, const MAX_MSG_SIZE: usize> Drop
    for SvcSender<MESSENGER, CALLBACK, MAX_MSG_SIZE>
where
    MESSENGER: Messenger,
    CALLBACK: Callback<MESSENGER>,
{
    fn drop(&mut self) {
        debug!("{} aborting receiver queue", self);
        self.recver_abort_handle.abort();
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
    pub async fn send(&self, msg: &MESSENGER::SendMsg) -> Result<(), Box<dyn Error + Send + Sync>> {
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
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    p1: std::marker::PhantomData<HANDLER>,
    p2: std::marker::PhantomData<CALLBACK>,
}

impl<HANDLER, CALLBACK, const MAX_MSG_SIZE: usize> Svc<HANDLER, CALLBACK, MAX_MSG_SIZE>
where
    HANDLER: Protocol,
    CALLBACK: Callback<HANDLER>,
{
    pub async fn new(
        addr: &str,
        callback: Arc<CALLBACK>,
        name: Option<&str>,
    ) -> Result<SvcSender<HANDLER, CALLBACK, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        let con_id = ConId::svc(name, addr, None);
        let lis = TcpListener::bind(&addr).await?;
        debug!("{} bound successfully", con_id);

        let senders = SvcSendersRef::new(Mutex::new(VecDeque::new()));

        let recver_abort_handle = tokio::spawn({
            let con_id = con_id.clone();
            let callback = Arc::clone(&callback);
            let senders = Arc::clone(&senders);
            async move {
                debug!("{} accept loop started", con_id);
                match Self::service_accept(lis, callback, senders, con_id.clone()).await {
                    Ok(()) => debug!("{} accept loop stopped", con_id),
                    Err(e) => error!("{} accept loop error: {:?}", con_id, e),
                }
            }
        })
        .abort_handle();

        Ok(SvcSender {
            con_id: con_id.clone(),
            senders: Arc::clone(&senders),
            recver_abort_handle,
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

    use super::*;
    use crate::unittest::setup::{model::*, protocol::*};
    use links_testing::unittest::setup;
    use tokio::time::Duration;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }
    const MAX_MSG_SIZE: usize = 128;
    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let logger = LoggerCallbackRef::<SvcMsgProtocol>::default();
        let svc = Svc::<_, _, MAX_MSG_SIZE>::new(&ADDR, Arc::clone(&logger), Some("unittest"))
            .await
            .unwrap();
        info!("{} ready", svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure();

        let clt_event_log = EventLogCallbackRef::<CltMsgProtocol>::default();
        let clt_callback = ChainCallbackRef::new(ChainCallback::new(vec![
            LoggerCallbackRef::default(),
            clt_event_log.clone(),
        ]));

        let svc_event_log = EventLogCallbackRef::<SvcMsgProtocol>::default();
        let svc_callback = ChainCallbackRef::new(ChainCallback::new(vec![
            LoggerCallbackRef::default(),
            svc_event_log.clone(),
        ]));

        let svc = Svc::<_, _, MAX_MSG_SIZE>::new(&ADDR, Arc::clone(&svc_callback), Some("venue"))
            .await
            .unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<CltMsgProtocol, _, MAX_MSG_SIZE>::new(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            Arc::clone(&clt_callback),
            Some("broker"),
        )
        .await
        .unwrap();
        info!("{} sender ready", clt);

        while !svc.is_accepted().await {}

        let inp_clt_msg = CltMsg::new(b"Hello Frm Client Msg");
        let inp_svc_msg = SvcMsg::new(b"Hello Frm Server Msg");
        clt.send(&inp_clt_msg).await.unwrap();
        svc.send(&inp_svc_msg).await.unwrap();

        let out_svc_msg = svc_event_log
            .find(
                |entry| match &entry.event {
                    Event::Recv(msg) => msg == &inp_clt_msg,
                    _ => false,
                },
                setup::net::default_find_timeout(),
            )
            .await;

        let out_clt_msg = clt_event_log
            .find(
                |entry| match &entry.event {
                    Event::Recv(msg) => msg == &inp_svc_msg,
                    _ => false,
                },
                setup::net::default_find_timeout(),
            )
            .await;

        info!("Found out_svc_msg: {:?}", out_svc_msg);
        info!("Found out_clt_msg: {:?}", out_clt_msg);
        assert_eq!(&inp_clt_msg, out_svc_msg.unwrap().try_into_recv().unwrap());
        assert_eq!(&inp_svc_msg, out_clt_msg.unwrap().try_into_recv().unwrap());
        info!("clt_event_log: {}", clt_event_log);
        info!("svc_event_log: {}", svc_event_log);
    }
}
