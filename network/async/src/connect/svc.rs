use std::{any::type_name, collections::VecDeque, error::Error, fmt::Display, sync::Arc};

use crate::prelude::*;
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex, task::AbortHandle};

use super::clt::{Clt, CltSender};

// pub type SvcReaderRef<M, FRAMER> = Arc<Mutex<Option<MessageRecver<M, FRAMER>>>>;
#[rustfmt::skip]
pub type SvcSendersRef<M, CALLBACK, const MAX_MSG_SIZE: usize> = Arc<Mutex<VecDeque<CltSender<M, CALLBACK, MAX_MSG_SIZE>>>>;

// pub type CallbackRef<HANDLER> = Arc<Mutex<impl Callback<HANDLER>>>;

#[derive(Debug)]
pub struct SvcSender<M, CALLBACK, const MAX_MSG_SIZE: usize>
where
    M: Messenger,
    CALLBACK: CallbackSendRecv<M>,
{
    con_id: ConId,
    senders: SvcSendersRef<M, CALLBACK, MAX_MSG_SIZE>,
    acceptor_abort_handle: AbortHandle,
}
impl<M, CALLBACK, const MAX_MSG_SIZE: usize> Display
    for SvcSender<M, CALLBACK, MAX_MSG_SIZE>
where
    M: Messenger,
    CALLBACK: CallbackSendRecv<M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg_name = type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        let clb_name = type_name::<CALLBACK>()
            .split('<')
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

impl<M, CALLBACK, const MAX_MSG_SIZE: usize> Drop
    for SvcSender<M, CALLBACK, MAX_MSG_SIZE>
where
    M: Messenger,
    CALLBACK: CallbackSendRecv<M>,
{
    fn drop(&mut self) {
        debug!("{} aborting receiver queue", self);
        self.acceptor_abort_handle.abort();
    }
}
impl<M, CALLBACK, const MAX_MSG_SIZE: usize> SvcSender<M, CALLBACK, MAX_MSG_SIZE>
where
    M: Messenger,
    CALLBACK: CallbackSendRecv<M>,
{
    pub async fn is_accepted(&self) -> bool {
        let senders = self.senders.lock().await;
        !senders.is_empty()
    }
    pub async fn send(&self, msg: &M::SendMsg) -> Result<(), Box<dyn Error + Send + Sync>> {
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
    pub fn con_id(&self) -> &ConId {
        &self.con_id
    }
}

#[derive(Debug)]
pub struct Svc<PROTOCOL, CALLBACK, const MAX_MSG_SIZE: usize>
where
    PROTOCOL: Protocol,
    CALLBACK: CallbackSendRecv<PROTOCOL>,
{
    p1: std::marker::PhantomData<PROTOCOL>,
    p2: std::marker::PhantomData<CALLBACK>,
}

impl<PROTOCOL, CALLBACK, const MAX_MSG_SIZE: usize> Svc<PROTOCOL, CALLBACK, MAX_MSG_SIZE>
where
    PROTOCOL: Protocol,
    CALLBACK: CallbackSendRecv<PROTOCOL>,
{
    pub async fn bind(
        addr: &str,
        callback: Arc<CALLBACK>,
        protocol: Option<PROTOCOL>,
        name: Option<&str>,
    ) -> Result<SvcSender<PROTOCOL, CALLBACK, MAX_MSG_SIZE>, Box<dyn Error + Send + Sync>> {
        let con_id = ConId::svc(name, addr, None);
        let lis = TcpListener::bind(&addr).await?;
        debug!("{} bound successfully", con_id);

        let senders = SvcSendersRef::new(Mutex::new(VecDeque::new()));

        let acceptor_abort_handle = tokio::spawn({
            let con_id = con_id.clone();
            let callback = Arc::clone(&callback);
            let senders = Arc::clone(&senders);
            async move {
                debug!("{} accept loop started", con_id);
                match Self::service_accept(lis, callback, protocol, senders, con_id.clone()).await {
                    Ok(()) => debug!("{} accept loop stopped", con_id),
                    Err(e) => error!("{} accept loop error: {:?}", con_id, e),
                }
            }
        })
        .abort_handle();

        Ok(SvcSender {
            con_id: con_id.clone(),
            senders: Arc::clone(&senders),
            acceptor_abort_handle,
        })
    }
    async fn service_accept(
        lis: TcpListener,
        callback: Arc<CALLBACK>,
        protocol: Option<PROTOCOL>,
        senders: SvcSendersRef<PROTOCOL, CALLBACK, MAX_MSG_SIZE>,
        con_id: ConId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let mut con_id = con_id.clone();
            con_id.set_local(stream.local_addr()?);
            con_id.set_peer(stream.peer_addr()?);

            let clt = Clt::<PROTOCOL, CALLBACK, MAX_MSG_SIZE>::from_stream(
                stream,
                Arc::clone(&callback),
                protocol.clone(),
                con_id.clone(),
            )
            .await;
            match clt {
                Ok(clt) => {
                    debug!("{} accepted", clt);
                    senders.lock().await.push_back(clt);
                }
                Err(e) => {
                    error!("{} accept error: {:?}", con_id, e);
                }
            }
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
        let svc = Svc::<_, _, MAX_MSG_SIZE>::bind(
            &ADDR,
            Arc::clone(&logger),
            Some(SvcMsgProtocol),
            Some("unittest"),
        )
        .await
        .unwrap();
        info!("{} ready", svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure_at(log::LevelFilter::Info);
        let event_store = EventStoreRef::<Msg>::default();
        // let clt_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        // let svc_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        let clt_callback =
            EventStoreProxyCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone());
        let svc_callback =
            EventStoreProxyCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone());

        let svc = Svc::<_, _, MAX_MSG_SIZE>::bind(
            &ADDR,
            svc_callback,
            Some(SvcMsgProtocol),
            Some("venue"),
        )
        .await
        .unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<_, _, MAX_MSG_SIZE>::connect(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            clt_callback,
            Some(CltMsgProtocol),
            Some("broker"),
        )
        .await
        .unwrap();
        info!("{} sender ready", clt);

        while !svc.is_accepted().await {}

        let inp_clt_msg = CltMsg::Dbg(CltMsgDebug::new(b"Hello Frm Client Msg"));
        let inp_svc_msg = SvcMsg::Dbg(SvcMsgDebug::new(b"Hello Frm Server Msg"));
        clt.send(&inp_clt_msg).await.unwrap();
        svc.send(&inp_svc_msg).await.unwrap();

        let out_svc_msg = event_store
            .find(
                |entry| match &entry.payload {
                    Dir::Recv(Msg::Clt(msg)) => msg == &inp_clt_msg,
                    _ => false,
                },
                setup::net::optional_find_timeout(),
            )
            .await
            .unwrap()
            .payload;
        // .into_t();
        // let out_svc_msg = event_store
        //     .find(
        //         |entry| match &entry.event {
        //             // Event::Recv(msg) => msg == &inp_svc_msg,
        //             Event::Recv(..) => true,
        //             // Event::<Msg>::Recv(msg) => msg == &inp_clt_msg,
        //             // _ => false,
        //         },
        //         setup::net::optional_find_timeout(),
        //     )
        //     .await;

        // let out_clt_msg = clt_event_store
        //     .find(
        //         |entry| match &entry.event {
        //             Event::Recv(msg) => msg == &inp_svc_msg,
        //             _ => false,
        //         },
        //         setup::net::default_find_timeout(),
        //     )
        //     .await;

        info!("Found out_svc_msg: {:?}", out_svc_msg);
        // info!("Found out_clt_msg: {:?}", out_clt_msg);
        // assert_eq!(&inp_clt_msg, out_svc_msg.unwrap().try_into_recv().unwrap());
        // assert_eq!(&inp_svc_msg, out_clt_msg.unwrap().try_into_recv().unwrap());
        // info!("clt_event_log: {}", clt_event_store);
        // info!("svc_event_log: {}", svc_event_store);
        // tokio::time::sleep(Duration::from_secs(1)).await;
        // info!("clt: {}", clt);
        // info!("svc: {}", svc);
        info!("event_store: {}", event_store);
    }
}
