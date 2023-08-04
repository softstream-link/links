use std::{
    any::type_name,
    fmt::{Debug, Display},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::{CallbackEvent, CallbackSendRecv, Dir};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<T> {
    pub con_id: ConId,
    pub instant: Instant,
    pub event: Dir<T>,
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
        let name = type_name::<T>().split("::").last().unwrap_or("Unknown");
        let events = self.lock();
        writeln!(f, "EventStore<{}, {}>", name, events.len())?;

        if events.len() == 1 {
            let entry1 = events.first().expect("Could Not Get First Entry");
            let idx = 0;
            let delta = format!("{:?}", Duration::from_secs(0));
            writeln!(f, "{:<04} Δ{: >15} {}", idx + 1, delta, entry1)?;
        }
        for (idx, pair) in events.windows(2).enumerate() {
            let entry1 = &pair[0];
            if idx == 0 {
                let delta = format!("{:?}", Duration::from_secs(0));
                writeln!(f, "{:<04} Δ{: >15} {}", idx + 1, delta, entry1)?;
            }

            let entry2 = &pair[1];
            let delta = entry2.instant - entry1.instant;
            let delta = format!("{:?}", delta);
            writeln!(f, "{:<04} Δ{: >15} {}", idx + 2, delta, entry2)?;
        }
        Ok(())
    }
}

pub type EventStoreCallbackRef<T, M> = Arc<EventStoreCallback<T, M>>;
#[derive(Debug)]
pub struct EventStoreCallback<T, M>
where
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    store: EventStoreRef<T>,
    phantom: std::marker::PhantomData<M>,
}

impl<T, M> Default for EventStoreCallback<T, M>
where
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
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
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn on_event(&self, cond_id: &crate::core::ConId, event: Dir<T>) {
        self.store.push(Entry {
            con_id: cond_id.clone(),
            instant: Instant::now(),
            event,
        })
    }
}
impl<T, M> Display for EventStoreCallback<T, M>
where
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventStoreCallback->")?;
        Display::fmt(&self.store, f)
    }
}
impl<T, M> CallbackSendRecv<M> for EventStoreCallback<T, M>
where
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
    M: Messenger,
{
    fn on_recv(&self, con_id: &crate::core::ConId, msg: <M as Messenger>::RecvMsg) {
        let entry = msg.into();
        self.on_event(con_id, Dir::Recv(entry));
    }
    fn on_send(&self, con_id: &crate::core::ConId, msg: &<M as Messenger>::SendMsg) {
        let entry = msg.clone().into();
        self.on_event(con_id, Dir::Send(entry));
    }
}

impl<T, M> EventStoreCallback<T, M>
where
    T: From<M::RecvMsg> + From<M::SendMsg> + Debug + Clone + Send + Sync + 'static,
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
        let event_clb = EventStoreCallback::<Msg, CltMsgProtocol>::new(Arc::clone(&event_store));

        #[allow(unused_assignments)]
        let mut clt_msg = CltMsg::Dbg(CltMsgDebug::new(format!("initialized").as_bytes()));
        for idx in 0..10 {
            let svc_msg = SvcMsg::Dbg(SvcMsgDebug::new(format!("hello  svc #{}", idx).as_bytes()));
            event_clb.on_recv(&Default::default(), svc_msg.clone());

            clt_msg = CltMsg::Dbg(CltMsgDebug::new(format!("hello  clt #{}", idx).as_bytes()));
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
