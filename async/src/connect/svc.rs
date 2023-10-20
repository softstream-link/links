use std::{any::type_name, collections::VecDeque, error::Error, fmt::Display, sync::Arc, time::Duration};

use crate::prelude::*;
use links_core::prelude::{CallbackSendRecvOld, ConId};
use log::{debug, error, warn};
use tokio::{net::TcpListener, sync::Mutex, task::AbortHandle};

use super::clt::{Clt, CltSender};

pub type SvcSendersRef<P, C, const MMS: usize> = Arc<Mutex<VecDeque<CltSender<P, C, MMS>>>>;

#[derive(Debug)]
pub struct SvcSender<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> {
    con_id: ConId,
    senders: SvcSendersRef<P, C, MMS>,
    acceptor_abort_handle: AbortHandle,
}
impl<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> SvcSender<P, C, MMS> {
    pub async fn is_accepted(&self) -> bool {
        let senders = self.senders.lock().await;
        !senders.is_empty()
    }
    pub async fn send(&self, msg: &mut P::SendT) -> Result<(), Box<dyn Error+Send+Sync>> {
        {
            let mut senders = self.senders.lock().await;
            for idx in 0..senders.len() {
                let sender = senders.pop_front().expect("senders can't be empty since we are in the loop");
                match sender.send(msg).await {
                    Ok(_) => {
                        senders.push_back(sender);
                        drop(senders);
                        return Ok(());
                    }
                    Err(err) => {
                        warn!("{} sender failure idx: {} evicting as disconnected err: {:?}", sender, idx, err);
                    }
                }
            }

            Err(format!("Not Connected senders len: {}", senders.len()).into()) // TODO better error type
        }
    }

    pub async fn is_connected(&self, timeout: Option<Duration>) -> bool {
        let senders = self.senders.lock().await;
        for sender in senders.iter() {
            if sender.is_connected(timeout).await {
                return true;
            }
        }
        false
    }
    pub fn con_id(&self) -> &ConId {
        &self.con_id
    }
}
impl<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> Display for SvcSender<P, C, MMS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use futures::executor::block_on;
        let clts = block_on(self.senders.lock())
            .iter()
            .map(|clt| format!("[{}]", clt.con_id().get_peer().unwrap()))
            .collect::<Vec<_>>()
            .join(", ");
        let con_id = format!("Svc({}@{}<-{})", self.con_id.name(), self.con_id.get_local().unwrap(), clts);
        #[rustfmt::skip]
        let name = {
            let mut protocol_full_name = type_name::<P>().split(['<','>']);
            format!("{} SvcSender<{}<{}>, {}, {}>", 
                con_id,
                protocol_full_name.next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"), 
                protocol_full_name.next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"),
                type_name::<C>().split('<').next().unwrap_or("Unknown").split("::").last().unwrap_or("Unknown"),
                MMS,
            )
        };
        write!(f, "{}", name)
    }
}
impl<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> Drop for SvcSender<P, C, MMS> {
    fn drop(&mut self) {
        debug!("{} aborting acceptor queue", self);
        self.acceptor_abort_handle.abort();
    }
}

#[derive(Debug)]
pub struct Svc<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> {
    phantom: std::marker::PhantomData<(P, C)>,
}
impl<P: Protocol, C: CallbackSendRecvOld<P>, const MMS: usize> Svc<P, C, MMS> {
    pub async fn bind(addr: &str, callback: Arc<C>, protocol: Option<Arc<P>>, name: Option<&str>) -> Result<SvcSender<P, C, MMS>, Box<dyn Error+Send+Sync>> {
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
    async fn service_accept(lis: TcpListener, callback: Arc<C>, protocol: Option<Arc<P>>, senders: SvcSendersRef<P, C, MMS>, con_id: ConId) -> Result<(), Box<dyn Error+Send+Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let mut con_id = con_id.clone();
            con_id.set_local(stream.local_addr()?);
            con_id.set_peer(stream.peer_addr()?);

            let clt = Clt::<P, C, MMS>::from_stream(stream, Arc::clone(&callback), protocol.clone(), con_id.clone()).await;
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

    use log::{info, Level};

    use super::*;
    use links_core::{
        prelude::LoggerCallbackOld,
        unittest::setup::{self, model::*},
    };

    use crate::{
        prelude::{EventStoreAsync, EventStoreCallback},
        unittest::setup::protocol::*,
    };

    use tokio::time::Duration;

    const MMS: usize = 128;
    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let logger = LoggerCallbackOld::new_ref(Level::Debug, Level::Debug);
        let svc = Svc::<_, _, MMS>::bind(setup::net::rand_avail_addr_port(), Arc::clone(&logger), Some(TestSvcMsgProtocol.into()), Some("unittest"))
            .await
            .unwrap();
        info!("{} ready", svc);
    }

    #[tokio::test]
    async fn test_svc_clt_connected() {
        setup::log::configure_level(log::LevelFilter::Debug);
        let addr = setup::net::rand_avail_addr_port();
        let event_store = EventStoreAsync::<TestMsg>::new_ref();
        // let clt_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, CltMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        // let svc_callback = ChainCallback::new_ref(vec![
        //     LoggerCallback::new_ref(log::Level::Warn),
        //     EventStoreProxyCallback::<Msg, SvcMsgProtocol>::new_ref(event_store.clone()),
        // ]);
        let clt_callback = EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new_ref(event_store.clone());
        let svc_callback = EventStoreCallback::<TestMsg, TestSvcMsgProtocol>::new_ref(event_store.clone());

        let svc = Svc::<_, _, MMS>::bind(addr, svc_callback, Some(TestSvcMsgProtocol.into()), Some("venue")).await.unwrap();

        info!("{} sender ready", svc);

        let clt = Clt::<_, _, MMS>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clt_callback,
            Some(TestCltMsgProtocol.into()),
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
            .find_recv("venue", |into| matches!(into, TestMsg::Clt(msg) if msg == &inp_clt_msg), setup::net::optional_find_timeout())
            .await
            .unwrap();

        let out_clt_msg = event_store
            .find_recv("broker", |into| matches!(into, TestMsg::Svc(msg) if msg == &inp_svc_msg), setup::net::optional_find_timeout())
            .await
            .unwrap();

        info!("Found out_svc_msg: {:?}", out_svc_msg);
        info!("Found out_clt_msg: {:?}", out_clt_msg);
        info!("clt: {}", clt);
        info!("svc: {}", svc);
        info!("event_store: {}", event_store);
        drop(clt);
        // TODO explore https://crates.io/crates/testing_logger to validate that drop did in fact work
        tokio::time::sleep(Duration::from_secs(1)).await; // sleep so that you see the drop(clt) loggin on log::debug!()
    }
}
