use std::{any::type_name, collections::VecDeque, error::Error, fmt::Display, sync::Arc};

use crate::prelude::*;
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex, task::AbortHandle};

use super::clt::{Clt, CltSender};

pub type SvcSendersRef<P, C, const MMS: usize> = Arc<Mutex<VecDeque<CltSender<P, C, MMS>>>>;

#[derive(Debug)]
pub struct SvcSender<P, C, const MMS: usize>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    con_id: ConId,
    senders: SvcSendersRef<P, C, MMS>,
    acceptor_abort_handle: AbortHandle,
}
impl<P, C, const MMS: usize> Display for SvcSender<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[rustfmt::skip]
        let name = {
            let mut protocol_full_name = type_name::<P>().split(['<','>']);
            format!("{} SvcSender<{}<{}>, {}, {}>", 
                match &self.con_id {ConId::Svc { name, local, .. } => format!("Svc({}@{})", name, local), _ => panic!("SvcSender has Invalid ConId: {:?}", self.con_id),},
                protocol_full_name.next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"), 
                protocol_full_name.next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"),
                type_name::<C>().split('<').next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"),
                MMS,
            )
        };
        write!(f, "{}", name)
    }
}

impl<P, C, const MMS: usize> Drop for SvcSender<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    fn drop(&mut self) {
        debug!("{} aborting acceptor queue", self);
        self.acceptor_abort_handle.abort();
    }
}
impl<P, C, const MMS: usize> SvcSender<P, C, MMS>
where
    P: Protocol,
    C: CallbackSendRecv<P>,
{
    pub async fn is_accepted(&self) -> bool {
        let senders = self.senders.lock().await;
        !senders.is_empty()
    }
    pub async fn send(&self, msg: &mut P::SendT) -> Result<(), Box<dyn Error+Send+Sync>> {
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
        protocol: Arc<P>,
        name: Option<&str>,
    ) -> Result<SvcSender<P, C, MMS>, Box<dyn Error+Send+Sync>> {
        Self::bind_opt_protocol(addr, callback, Some(protocol), name).await
    }
    pub async fn bind_no_protocol(
        addr: &str,
        callback: Arc<C>,
        name: Option<&str>,
    ) -> Result<SvcSender<P, C, MMS>, Box<dyn Error+Send+Sync>> {
        Self::bind_opt_protocol(addr, callback, None, name).await
    }
    async fn bind_opt_protocol(
        addr: &str,
        callback: Arc<C>,
        protocol: Option<Arc<P>>,
        name: Option<&str>,
    ) -> Result<SvcSender<P, C, MMS>, Box<dyn Error+Send+Sync>> {
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
        protocol: Option<Arc<P>>,
        senders: SvcSendersRef<P, C, MMS>,
        con_id: ConId,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
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
    use crate::{
        callbacks::eventstore::EventStore,
        unittest::setup::{model::*, protocol::*},
    };
    use links_testing::unittest::setup;
    use tokio::time::Duration;

    lazy_static! {
        static ref ADDR: &'static str = setup::net::default_addr();
    }
    const MMS: usize = 128;
    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let logger = LoggerCallback::new_ref(Level::Debug);
        let svc = Svc::<_, _, MMS>::bind(
            &ADDR,
            Arc::clone(&logger),
            TestSvcMsgProtocol.into(),
            Some("unittest"),
        )
        .await
        .unwrap();
        info!("{} ready", svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure_at(log::LevelFilter::Debug);
        let event_store = EventStore::<TestMsg>::new_ref();
        // let clt_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        // let svc_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        let clt_callback =
            EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new_ref(event_store.clone());
        let svc_callback =
            EventStoreCallback::<TestMsg, TestSvcMsgProtocol>::new_ref(event_store.clone());

        let svc = Svc::<_, _, MMS>::bind(
            &ADDR,
            svc_callback,
            TestSvcMsgProtocol.into(),
            Some("venue"),
        )
        .await
        .unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<_, _, MMS>::connect(
            &ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clt_callback,
            TestCltMsgProtocol.into(),
            Some("broker"),
        )
        .await
        .unwrap();
        info!("{} sender ready", clt);

        while !svc.is_accepted().await {}

        let mut inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let mut inp_svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
        clt.send(&mut inp_clt_msg).await.unwrap();
        svc.send(&mut inp_svc_msg).await.unwrap();

        let out_svc_msg = event_store
            .find(
                |entry| match &entry.event {
                    Dir::Recv(TestMsg::Clt(msg)) => msg == &inp_clt_msg,
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
                    Dir::Recv(TestMsg::Svc(msg)) => msg == &inp_svc_msg,
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
        drop(clt);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
