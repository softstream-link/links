use std::{
    any::type_name,
    fmt::{Debug, Display},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Local};
use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::{CallbackEvent, CallbackSendRecv, Dir};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<T> {
    pub con_id: ConId,
    pub instant: Instant,
    pub time: SystemTime,
    pub event: Dir<T>,
}
impl<T> Entry<T>{
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
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{:?}", self.con_id, self.event)
    }
}

pub type EventStoreRef<T> = Arc<EventStore<T>>;

#[derive(Debug)]
pub struct EventStore<T>
where
    T: Debug + Clone + Send + Sync + 'static,
{
    store: Mutex<Vec<Entry<T>>>,
}
impl<T> Default for EventStore<T>
where
    T: Debug + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}
impl<T> EventStore<T>
where
    T: Debug + Clone + Send + Sync + 'static,
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
    pub async fn find<P>(&self, mut predicate: P, timeout: Option<Duration>) -> Option<Entry<T>>
    where
        P: FnMut(&Entry<T>) -> bool,
    {
        let now = Instant::now();
        let timeout = timeout.unwrap_or_else(|| Duration::from_secs(0));
        loop {
            {
                // this scope will drop the lock before the next await
                let events = self.lock();
                let opt = events.iter().rev().find(|entry| predicate(*entry));
                if opt.is_some() {
                    return opt.cloned();
                }
            }

            if now.elapsed() > timeout {
                break;
            }
            yield_now().await;
        }
        None
    }
    pub fn last(&self) -> Option<Entry<T>> {
        self.lock().last().cloned()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }
}
impl<T> Display for EventStore<T>
where
    T: Debug + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[rustfmt::skip]
        fn writeln<T: Debug>(f: &mut std::fmt::Formatter<'_>, count: usize, delta_window: Duration, entry: &Entry<T>) -> std::fmt::Result {    
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
            writeln(f,  1, Duration::from_secs(0), entry1)?;
        }

        for (idx, pair) in events.windows(2).enumerate() {
            writeln(f, idx + 2, pair[1].instant - pair[0].instant, &pair[1])?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    store: EventStoreRef<T>,
    phantom: std::marker::PhantomData<M>,
}
impl<T, M> EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    pub fn new(store: EventStoreRef<T>) -> Self {
        Self {
            store,
            phantom: std::marker::PhantomData,
        }
    }
    pub fn new_ref(store: EventStoreRef<T>) -> Arc<Self> {
        Arc::new(Self {
            store,
            phantom: std::marker::PhantomData,
        })
    }
}
impl<T, M> Default for EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn default() -> Self {
        Self {
            store: Default::default(),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<T, M> CallbackEvent<T, M> for EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn on_event(&self, cond_id: &crate::core::ConId, event: Dir<T>) {
        self.store.push(Entry {
            con_id: cond_id.clone(),
            instant: Instant::now(),
            time: SystemTime::now(),
            event,
        })
    }
}
impl<T, M> Display for EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventStoreCallback->")?;
        Display::fmt(&self.store, f)
    }
}
impl<T, M> CallbackSendRecv<M> for EventStoreCallback<T, M>
where
    T: From<M::RecvT> + From<M::SendT> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn on_recv(&self, con_id: &crate::core::ConId, msg: <M as Messenger>::RecvT) {
        let entry = msg.into();
        self.on_event(con_id, Dir::Recv(entry));
    }
    fn on_send(&self, con_id: &crate::core::ConId, msg: &<M as Messenger>::SendT) {
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
        let event_clb = EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new(Arc::clone(&event_store));

        #[allow(unused_assignments)]
        let mut clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(format!("initialized").as_bytes()));
        for idx in 0..10 {
            let svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(format!("hello  svc #{}", idx).as_bytes()));
            event_clb.on_recv(&Default::default(), svc_msg.clone());

            clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(format!("hello  clt #{}", idx).as_bytes()));
            event_clb.on_send(&Default::default(), &clt_msg);
        }
        info!("event_clb: {}", event_clb);
        let found = event_store.find(|_| true, None).await;
        info!("found: {:?}", found);
        let last = event_store.last();
        info!("last: {:?}", last);
        assert_eq!(last, found);
        match found.unwrap().event {
            Dir::Send(msg) => assert_eq!(msg, msg),
            _ => panic!("unexpected event"),
        }
    }
}
