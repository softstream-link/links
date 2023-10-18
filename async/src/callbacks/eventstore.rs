use std::{
    any::type_name,
    fmt::{Debug, Display},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Local};
use links_core::prelude::{ConId, EntryOld, MessengerOld};
use tokio::{runtime::Runtime, task::yield_now};

use links_core::prelude::{CallbackEvent, CallbackSendRecvOld, DirOld};

pub type EventStoreAsyncRef<T> = Arc<EventStoreAsync<T>>;

#[derive(Debug)]
pub struct EventStoreSync<T: Debug+Clone+Send+Sync+'static> {
    store: EventStoreAsyncRef<T>,
    runtime: Arc<Runtime>,
}
impl<T: Debug+Clone+Send+Sync+'static> EventStoreSync<T> {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            store: EventStoreAsyncRef::default(),
            runtime,
        }
    }

    pub fn async_ref(&self) -> EventStoreAsyncRef<T> {
        Arc::clone(&self.store)
    }
    pub fn find<P: FnMut(&EntryOld<T>) -> bool>(
        &self,
        con_id_name: &str,
        predicate: P,
        timeout: Option<Duration>,
    ) -> Option<EntryOld<T>> {
        self.runtime
            .block_on(self.store.find(con_id_name, predicate, timeout))
    }
    pub fn find_recv<P: FnMut(&T) -> bool>(
        &self,
        con_id_name: &str,
        predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T> {
        self.runtime
            .block_on(self.store.find_recv(con_id_name, predicate, timeout))
    }
    pub fn find_send<P: FnMut(&T) -> bool>(
        &self,
        con_id_name: &str,
        predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T> {
        self.runtime
            .block_on(self.store.find_send(con_id_name, predicate, timeout))
    }
    pub fn last(&self) -> Option<EntryOld<T>> {
        self.store.last()
    }
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}
impl<T: Debug+Clone+Send+Sync+'static> Display for EventStoreSync<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.store, f)
    }
}

#[derive(Debug)]
pub struct EventStoreAsync<T: Debug+Clone+Send+Sync+'static> {
    store: Mutex<Vec<EntryOld<T>>>,
}
impl<T: Debug+Clone+Send+Sync+'static> EventStoreAsync<T> {
    pub fn new_ref() -> EventStoreAsyncRef<T> {
        Arc::new(Self::default())
    }
    fn lock(&self) -> MutexGuard<'_, Vec<EntryOld<T>>> {
        let grd = self.store.lock().expect("Could Not Lock EventStore");
        grd
    }
    pub fn push(&self, e: EntryOld<T>) {
        self.lock().push(e);
    }
    pub async fn find<P: FnMut(&EntryOld<T>) -> bool>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<EntryOld<T>> {
        let now = Instant::now();
        let timeout = timeout.unwrap_or_else(|| Duration::from_secs(0));
        loop {
            {
                // this scope will drop the lock before the next await
                let events = self.lock();
                let opt = events
                    .iter()
                    .rev()
                    .find(|entry| entry.con_id.name() == con_id_name && predicate(*entry));
                if opt.is_some() {
                    return opt.cloned(); // because the result is behind a mutex must clone in order to return
                }
            }

            if now.elapsed() > timeout {
                break;
            }
            yield_now().await;
        }
        None
    }
    pub async fn find_recv<P: FnMut(&T) -> bool>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T> {
        let entry = self
            .find(
                con_id_name,
                |entry| match entry.event {
                    DirOld::Recv(ref t) => match predicate(t) {
                        true => true,
                        false => false,
                    },
                    _ => false,
                },
                timeout,
            )
            .await;
        match entry {
            Some(EntryOld {
                event: DirOld::Recv(t),
                ..
            }) => Some(t),
            _ => None,
        }
    }
    pub async fn find_send<P: FnMut(&T) -> bool>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T> {
        let entry = self
            .find(
                con_id_name,
                |entry| match entry.event {
                    DirOld::Send(ref t) => match predicate(t) {
                        true => true,
                        false => false,
                    },
                    _ => false,
                },
                timeout,
            )
            .await;
        match entry {
            Some(EntryOld {
                event: DirOld::Send(t),
                ..
            }) => Some(t),
            _ => None,
        }
    }
    pub fn last(&self) -> Option<EntryOld<T>> {
        self.lock().last().cloned()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}
impl<T: Debug+Clone+Send+Sync+'static> Default for EventStoreAsync<T> {
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}
impl<T: Debug+Clone+Send+Sync+'static> Display for EventStoreAsync<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn writeln<T: Debug>(
            f: &mut std::fmt::Formatter<'_>,
            count: usize,
            delta_window: Duration,
            entry: &EntryOld<T>,
        ) -> std::fmt::Result {
            let dt: DateTime<Local> = entry.time.into();
            let dt = &dt.format("%T.%f").to_string()[..15];
            writeln!(f, "{:<04} Δ{: >15?} {} {}", count, delta_window, dt, entry)?;
            Ok(())
        }
        let name = type_name::<T>().split("::").last().unwrap_or("Unknown");
        let events = self.lock();
        writeln!(f, "EventStore<{}, {}>", name, events.len())?;

        if !events.is_empty() {
            let entry1 = events.first().expect("Could Not Get First Entry");
            writeln(f, 1, Duration::from_secs(0), entry1)?;
        }

        for (idx, pair) in events.windows(2).enumerate() {
            writeln(f, idx + 2, pair[1].instant - pair[0].instant, &pair[1])?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    store: EventStoreAsyncRef<INTO>,
    phantom: std::marker::PhantomData<M>,
}
impl<INTO, M> EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    pub fn new(store: EventStoreAsyncRef<INTO>) -> Self {
        Self {
            store,
            phantom: std::marker::PhantomData,
        }
    }
    pub fn new_ref(store: EventStoreAsyncRef<INTO>) -> Arc<Self> {
        Arc::new(Self {
            store,
            phantom: std::marker::PhantomData,
        })
    }
}
impl<INTO, M> Default for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    fn default() -> Self {
        Self {
            store: Default::default(),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<INTO, M> CallbackEvent<INTO, M> for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    fn on_event(&self, cond_id: &ConId, event: DirOld<INTO>) {
        self.store.push(EntryOld {
            con_id: cond_id.clone(),
            instant: Instant::now(),
            time: SystemTime::now(),
            event,
        })
    }
}
impl<INTO, M> Display for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventStoreCallback->")?;
        Display::fmt(&self.store, f)
    }
}
impl<INTO, M> CallbackSendRecvOld<M> for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: MessengerOld,
{
    fn on_recv(&self, con_id: &ConId, msg: <M as MessengerOld>::RecvT) {
        let entry = msg.into();
        self.on_event(con_id, DirOld::Recv(entry));
    }
    fn on_send(&self, con_id: &ConId, msg: &<M as MessengerOld>::SendT) {
        let entry = msg.clone().into();
        self.on_event(con_id, DirOld::Send(entry));
    }
}

#[cfg(test)]
mod test {

    use log::info;
    use tokio::runtime::Builder;

    use crate::unittest::setup::protocol::*;
    use links_core::unittest::setup::{self, model::*};

    use super::*;

    #[test]
    fn test_callback() {
        setup::log::configure();

        let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());

        // let event_store_async = EventStoreAsync::new_ref();
        let event_store = EventStoreSync::new(runtime);
        let clt_clb =
            EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new(event_store.async_ref());
        let svc_clb =
            EventStoreCallback::<TestMsg, TestSvcMsgProtocol>::new(event_store.async_ref());

        let svc_on_recv_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"SVC: on_recv Message"));
        let svc_on_send_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"SVC: on_send Message"));
        svc_clb.on_recv(
            &ConId::svc(Some("svc"), "0.0.0.0:0", None),
            svc_on_recv_msg.clone(),
        );
        svc_clb.on_send(
            &ConId::svc(Some("svc"), "0.0.0.0:0", None),
            &svc_on_send_msg,
        );

        let clt_on_recv_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"CLT: on_recv Message"));
        let clt_on_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"CLT: on_send Message"));
        clt_clb.on_recv(
            &ConId::clt(Some("clt"), None, "0.0.0.0:0"),
            clt_on_recv_msg.clone(),
        );
        clt_clb.on_send(
            &ConId::clt(Some("clt"), None, "0.0.0.0:0"),
            &clt_on_send_msg,
        );

        info!("event_clb: {}", clt_clb);

        // Entry find
        let last_svc = event_store.find("svc", |_| true, None).unwrap();
        info!("last_svc: {:?}", last_svc);
        assert_eq!(
            last_svc.event,
            DirOld::Send(TestMsg::Svc(svc_on_send_msg.clone()))
        );

        let last_clt = event_store.find("clt", |_| true, None).unwrap();
        info!("last_clt: {:?}", last_clt);
        assert_eq!(
            last_clt.event,
            DirOld::Send(TestMsg::Clt(clt_on_send_msg.clone()))
        );
        let last_entry = event_store.last().unwrap();
        info!("last_entry: {:?}", last_entry);
        assert_eq!(last_entry, last_clt);

        // RECV find unpacks the event into TestMsg
        let svc_recv = event_store.find_recv(
            "svc",
            |msg| matches!(msg, TestMsg::Clt(TestCltMsg::Dbg(TestCltMsgDebug{text, ..})) if text == &b"SVC: on_recv Message".as_slice().into() ),
            None
        );
        info!("svc_recv: {:?}", svc_recv);
        assert_eq!(svc_recv.unwrap(), TestMsg::Clt(svc_on_recv_msg));

        // SEND find unpacks the event into TestMsg
        let svc_send = event_store.find_send(
            "svc",
            |msg| matches!(msg, TestMsg::Svc(TestSvcMsg::Dbg(TestSvcMsgDebug{text, ..})) if text == &b"SVC: on_send Message".as_slice().into() ),
            None
        );
        info!("svc_send: {:?}", svc_send);
        assert_eq!(svc_send.unwrap(), TestMsg::Svc(svc_on_send_msg));

        // RECV find unpacks the event into TestMsg
        let clt_recv = event_store.find_recv(
            "clt",
            |msg| matches!(msg, TestMsg::Svc(TestSvcMsg::Dbg(TestSvcMsgDebug{text, ..})) if text == &b"CLT: on_recv Message".as_slice().into() ),
            None
        );
        info!("clt_recv: {:?}", clt_recv);
        assert_eq!(clt_recv.unwrap(), TestMsg::Svc(clt_on_recv_msg));

        // SEND find unpacks the event into TestMsg
        let clt_send = event_store.find_send(
            "clt",
            |msg| matches!(msg, TestMsg::Clt(TestCltMsg::Dbg(TestCltMsgDebug{text, ..})) if text == &b"CLT: on_send Message".as_slice().into() ),
            None
        );
        info!("clt_send: {:?}", clt_send);
        assert_eq!(clt_send.unwrap(), TestMsg::Clt(clt_on_send_msg));
    }
}
