use std::{
    any::type_name,
    fmt::{Debug, Display},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Local};
use tokio::task::yield_now;

use crate::core::{conid::ConId, Messenger};

use super::{CallbackEvent, CallbackSendRecv, Dir};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<T> {
    pub con_id: ConId,
    pub instant: Instant,
    pub time: SystemTime,
    pub event: Dir<T>,
}
impl<T> Entry<T> {
    pub fn unwrap_recv_event(self) -> T {
        match self.event {
            Dir::Recv(t) => t,
            Dir::Send(_) => panic!("Entry::try_into_recv: Not a Dir::Recv variant"),
        }
    }
    pub fn unwrap_send_event(self) -> T {
        match self.event {
            Dir::Recv(_) => panic!("Entry::try_into_send: Not a Dir::Send variant"),
            Dir::Send(t) => t,
        }
    }
}

impl<T> Display for Entry<T>
where T: Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{:?}", self.con_id, self.event)
    }
}

pub type EventStoreRef<T> = Arc<EventStore<T>>;

#[derive(Debug)]
pub struct EventStore<T>
where T: Debug+Clone+Send+Sync+'static
{
    store: Mutex<Vec<Entry<T>>>,
}
impl<T> Default for EventStore<T>
where T: Debug+Clone+Send+Sync+'static
{
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}
impl<T> EventStore<T>
where T: Debug+Clone+Send+Sync+'static
{
    pub fn new_ref() -> EventStoreRef<T> {
        Arc::new(Self::default())
    }
    fn lock(&self) -> MutexGuard<'_, Vec<Entry<T>>> {
        let grd = self.store.lock().expect("Could Not Lock EventStore");
        grd
    }
    pub fn push(&self, e: Entry<T>) {
        self.lock().push(e);
    }
    pub async fn find<P>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<Entry<T>>
    where
        P: FnMut(&Entry<T>) -> bool,
    {
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
                    return opt.cloned(); // because the resutl is behind a mutex must clone in order to return
                }
            }

            if now.elapsed() > timeout {
                break;
            }
            yield_now().await;
        }
        None
    }
    pub async fn find_recv<P>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T>
    where
        P: FnMut(&T) -> bool,
    {
        let entry = self
            .find(
                con_id_name,
                |entry| match entry.event {
                    Dir::Recv(ref t) => match predicate(t) {
                        true => true,
                        false => false,
                    },
                    _ => false,
                },
                timeout,
            )
            .await;
        match entry {
            Some(Entry {
                event: Dir::Recv(t),
                ..
            }) => Some(t),
            _ => None,
        }
    }
    pub async fn find_send<P>(
        &self,
        con_id_name: &str,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<T>
    where
        P: FnMut(&T) -> bool,
    {
        let entry = self
            .find(
                con_id_name,
                |entry| match entry.event {
                    Dir::Send(ref t) => match predicate(t) {
                        true => true,
                        false => false,
                    },
                    _ => false,
                },
                timeout,
            )
            .await;
        match entry {
            Some(Entry {
                event: Dir::Send(t),
                ..
            }) => Some(t),
            _ => None,
        }
    }
    pub fn last(&self) -> Option<Entry<T>> {
        self.lock().last().cloned()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}
impl<T> Display for EventStore<T>
where T: Debug+Clone+Send+Sync+'static
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn writeln<T: Debug>(
            f: &mut std::fmt::Formatter<'_>,
            count: usize,
            delta_window: Duration,
            entry: &Entry<T>,
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
    M: Messenger,
{
    store: EventStoreRef<INTO>,
    phantom: std::marker::PhantomData<M>,
}
impl<INTO, M> EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: Messenger,
{
    pub fn new(store: EventStoreRef<INTO>) -> Self {
        Self {
            store,
            phantom: std::marker::PhantomData,
        }
    }
    pub fn new_ref(store: EventStoreRef<INTO>) -> Arc<Self> {
        Arc::new(Self {
            store,
            phantom: std::marker::PhantomData,
        })
    }
}
impl<INTO, M> Default for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: Messenger,
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
    M: Messenger,
{
    fn on_event(&self, cond_id: &ConId, event: Dir<INTO>) {
        self.store.push(Entry {
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
    M: Messenger,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventStoreCallback->")?;
        Display::fmt(&self.store, f)
    }
}
impl<INTO, M> CallbackSendRecv<M> for EventStoreCallback<INTO, M>
where
    INTO: From<M::RecvT>+From<M::SendT>+Debug+Clone+Send+Sync+'static,
    M: Messenger,
{
    fn on_recv(&self, con_id: &ConId, msg: <M as Messenger>::RecvT) {
        let entry = msg.into();
        self.on_event(con_id, Dir::Recv(entry));
    }
    fn on_send(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {
        let entry = msg.clone().into();
        self.on_event(con_id, Dir::Send(entry));
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    use super::*;

    #[tokio::test]
    async fn test_event_store() {
        setup::log::configure();
        let event_store = EventStore::new_ref();
        let clt_clb = EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new(event_store.clone());
        let svc_clb = EventStoreCallback::<TestMsg, TestSvcMsgProtocol>::new(event_store.clone());

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
        let last_svc = event_store.find("svc", |_| true, None).await.unwrap();
        info!("last_svc: {:?}", last_svc);
        assert_eq!(
            last_svc.event,
            Dir::Send(TestMsg::Svc(svc_on_send_msg.clone()))
        );

        let last_clt = event_store.find("clt", |_| true, None).await.unwrap();
        info!("last_clt: {:?}", last_clt);
        assert_eq!(
            last_clt.event,
            Dir::Send(TestMsg::Clt(clt_on_send_msg.clone()))
        );
        let last_entry = event_store.last().unwrap();
        info!("last_entry: {:?}", last_entry);
        assert_eq!(last_entry, last_clt);

        // RECV find upacks the event into TestMsg
        let svc_recv = event_store.find_recv(
            "svc",
            |msg| matches!(msg, TestMsg::Clt(TestCltMsg::Dbg(TestCltMsgDebug{text, ..})) if text == &b"SVC: on_recv Message".into() ),
            None
        ).await;
        info!("svc_recv: {:?}", svc_recv);
        assert_eq!(svc_recv.unwrap(), TestMsg::Clt(svc_on_recv_msg));

        // SEND find upacks the event into TestMsg
        let svc_send = event_store.find_send(
            "svc",
            |msg| matches!(msg, TestMsg::Svc(TestSvcMsg::Dbg(TestSvcMsgDebug{text, ..})) if text == &b"SVC: on_send Message".into() ),
            None
        ).await;
        info!("svc_send: {:?}", svc_send);
        assert_eq!(svc_send.unwrap(), TestMsg::Svc(svc_on_send_msg));

        // RECV find upacks the event into TestMsg
        let clt_recv = event_store.find_recv(
            "clt",
            |msg| matches!(msg, TestMsg::Svc(TestSvcMsg::Dbg(TestSvcMsgDebug{text, ..})) if text == &b"CLT: on_recv Message".into() ),
            None
        ).await;
        info!("clt_recv: {:?}", clt_recv);
        assert_eq!(clt_recv.unwrap(), TestMsg::Svc(clt_on_recv_msg));

        // SEND find upacks the event into TestMsg
        let clt_send = event_store.find_send(
            "clt",
            |msg| matches!(msg, TestMsg::Clt(TestCltMsg::Dbg(TestCltMsgDebug{text, ..})) if text == &b"CLT: on_send Message".into() ),
            None
        ).await;
        info!("clt_send: {:?}", clt_send);
        assert_eq!(clt_send.unwrap(), TestMsg::Clt(clt_on_send_msg));
    }
}
