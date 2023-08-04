use std::{any::type_name, collections::VecDeque, error::Error, fmt::Display, sync::Arc};

use crate::prelude::*;
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex, task::AbortHandle};

use super::clt::{Clt, CltSender};

// pub type SvcReaderRef<M, F> = Arc<Mutex<Option<MessageRecver<M, F>>>>;
#[rustfmt::skip]
pub type SvcSendersRef<M, C, const MMS: usize> = Arc<Mutex<VecDeque<CltSender<M, C, MMS>>>>;

// pub type CallbackRef<HANDLER> = Arc<Mutex<impl Callback<HANDLER>>>;

#[derive(Debug)]
pub struct SvcSender<M, C, const MMS: usize>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    con_id: ConId,
    senders: SvcSendersRef<M, C, MMS>,
    acceptor_abort_handle: AbortHandle,
}
impl<M, C, const MMS: usize> Display for SvcSender<M, C, MMS>
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
        let con_name = match &self.con_id {
            ConId::Svc { name, local, .. } => format!("Svc({}@{})", name, local),
            _ => panic!("SvcSender has Invalid ConId: {:?}", self.con_id),
        };
        write!(
            f,
            "{} SvcSender<{}, {}, {}>",
            con_name, msg_name, clb_name, MMS
        )
    }
}

impl<M, C, const MMS: usize> Drop for SvcSender<M, C, MMS>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    fn drop(&mut self) {
        debug!("{} aborting receiver queue", self);
        self.acceptor_abort_handle.abort();
    }
}
impl<M, C, const MMS: usize> SvcSender<M, C, MMS>
where
    M: Messenger,
    C: CallbackSendRecv<M>,
{
    pub async fn is_accepted(&self) -> bool {
        let senders = self.senders.lock().await;
        !senders.is_empty()
    }
    pub async fn send(&self, msg: &M::SendT) -> Result<(), Box<dyn Error + Send + Sync>> {
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
pub struct Svc<P, C, const MMS: usize>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    p1: std::marker::PhantomData<P>,
    p2: std::marker::PhantomData<C>,
}

impl<P, C, const MMS: usize> Svc<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    pub async fn bind(
        addr: &str,
        callback: Arc<C>,
        protocol: Option<P>,
        name: Option<&str>,
    ) -> Result<SvcSender<P, C, MMS>, Box<dyn Error + Send + Sync>> {
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
        callback: Arc<C>,
        protocol: Option<P>,
        senders: SvcSendersRef<P, C, MMS>,
        con_id: ConId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let mut con_id = con_id.clone();
            con_id.set_local(stream.local_addr()?);
            con_id.set_peer(stream.peer_addr()?);

            let clt = Clt::<P, C, MMS>::from_stream(
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
    use log::{info, Level};

    use super::*;
    use crate::{unittest::setup::{model::*, protocol::*}, callbacks::eventstore::EventStore};
    use links_testing::unittest::setup;
    use tokio::time::Duration;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }
    const MMS: usize = 128;
    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let logger = LoggerCallback::new_ref(Level::Debug);
        let svc = Svc::<_, _, MMS>::bind(
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
        let event_store = EventStore::<Msg>::new_ref();
        // let clt_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        // let svc_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        let clt_callback =
            EventStoreCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone());
        let svc_callback =
            EventStoreCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone());

        let svc = Svc::<_, _, MMS>::bind(&ADDR, svc_callback, Some(SvcMsgProtocol), Some("venue"))
            .await
            .unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<_, _, MMS>::connect(
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
                |entry| match &entry.event {
                    Dir::Recv(Msg::Clt(msg)) => msg == &inp_clt_msg,
                    _ => false,
                },
                setup::net::optional_find_timeout(),
            )
            .await
            .unwrap()
            .event;

        let out_clt_msg = event_store
            .find(
                |entry| match &entry.event {
                    Dir::Recv(Msg::Svc(msg)) => msg == &inp_svc_msg,
                    _ => false,
                },
                setup::net::optional_find_timeout(),
            )
            .await
            .unwrap()
            .event;

        info!("Found out_svc_msg: {:?}", out_svc_msg);
        info!("Found out_clt_msg: {:?}", out_clt_msg);
        info!("clt: {}", clt);
        info!("svc: {}", svc);
        info!("event_store: {}", event_store);
    }
}
