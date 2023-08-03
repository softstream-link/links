use std::{
    any::type_name,
    fmt::{Debug, Display},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::{CallbackEvent, CallbackSendRecv, Event};

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<TARGET> {
    pub con_id: ConId,
    pub instant: Instant,
    pub event: Event<TARGET>,
}

impl<TARGET> Display for Entry<TARGET>
where
    TARGET: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{:?}", self.con_id, self.event)
    }
}

pub type EventStoreRef<TARGET> = Arc<EventStore<TARGET>>;

#[derive(Debug)]
pub struct EventStore<TARGET>
where
    TARGET: Debug + Clone + Send + Sync + 'static,
{
    store: Mutex<Vec<Entry<TARGET>>>,
}
impl<TARGET> Default for EventStore<TARGET>
where
    TARGET: Debug + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            store: Default::default(),
        }
    }
}
impl<TARGET> EventStore<TARGET>
where
    TARGET: Debug + Clone + Send + Sync + 'static,
{
    pub fn new_ref() -> EventStoreRef<TARGET> {
        Arc::new(Self::default())
    }
    fn lock(&self) -> MutexGuard<'_, Vec<Entry<TARGET>>> {
        let grd = self.store.lock().expect("Could Not Lock EventStore");
        grd
    }
    pub fn push(&self, e: Entry<TARGET>) {
        let mut events = self.lock();
        events.push(e);
    }
    pub async fn find<P>(
        &self,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<Entry<TARGET>>
    where
        P: FnMut(&Entry<TARGET>) -> bool,
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
    pub fn last(&self) -> Option<Entry<TARGET>> {
        let events = self.lock();
        events.last().cloned()
    }
}
impl<TARGET> Display for EventStore<TARGET>
where
    TARGET: Debug + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<TARGET>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
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

pub type EventStoreCallbackRef<TARGET, MESSENGER> = Arc<EventStoreProxyCallback<TARGET, MESSENGER>>;
#[derive(Debug)]
pub struct EventStoreProxyCallback<TARGET, MESSENGER: Messenger>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
{
    store: EventStoreRef<TARGET>,
    phantom: std::marker::PhantomData<MESSENGER>,
}

impl<TARGET, MESSENGER> Default for EventStoreProxyCallback<TARGET, MESSENGER>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
    MESSENGER: Messenger,
{
    fn default() -> Self {
        Self {
            store: Default::default(), // EventStoreRef::new(EventStore::<TARGET>::default()),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<TARGET, MESSENGER> CallbackEvent<TARGET, MESSENGER> for EventStoreProxyCallback<TARGET, MESSENGER>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
    MESSENGER: Messenger,
{
    fn on_event(&self, cond_id: &crate::core::ConId, event: Event<TARGET>) {
        self.store.push(Entry {
            con_id: cond_id.clone(),
            instant: Instant::now(),
            event,
        })
    }
}
impl<TARGET, MESSENGER> Display for EventStoreProxyCallback<TARGET, MESSENGER>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
    MESSENGER: Messenger,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventStoreCallback->")?;
        Display::fmt(&self.store, f)
    }
}
impl<TARGET, MESSENGER> CallbackSendRecv<MESSENGER> for EventStoreProxyCallback<TARGET, MESSENGER>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
    MESSENGER: Messenger,
{
    fn on_recv(&self, con_id: &crate::core::ConId, msg: <MESSENGER as Messenger>::RecvMsg) {
        let entry = msg.into();
        self.on_event(con_id, Event::Recv(entry));
    }
    fn on_send(&self, con_id: &crate::core::ConId, msg: &<MESSENGER as Messenger>::SendMsg) {
        let entry = msg.clone().into();
        self.on_event(con_id, Event::Send(entry));
    }
}

impl<TARGET, MESSENGER> EventStoreProxyCallback<TARGET, MESSENGER>
where
    TARGET:
        From<MESSENGER::RecvMsg> + From<MESSENGER::SendMsg> + Debug + Clone + Send + Sync + 'static,
    MESSENGER: Messenger,
{
    pub fn new(store: EventStoreRef<TARGET>) -> Self {
        Self {
            store,
            phantom: std::marker::PhantomData,
        }
    }
    pub fn new_ref(store: EventStoreRef<TARGET>) -> Arc<Self> {
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
        let event_clb = EventStoreProxyCallback::<Msg, CltMsgProtocol>::new(Arc::clone(&event_store));

        #[allow(unused_assignments)]
        let mut clt_msg = CltMsg::Dbg(CltDebugMsg::new(format!("initialized").as_bytes()));
        for idx in 0..10 {
            let svc_msg = SvcMsg::Dbg(SvcDebugMsg::new(format!("hello  svc #{}", idx).as_bytes()));
            event_clb.on_recv(&Default::default(), svc_msg.clone());

            clt_msg = CltMsg::Dbg(CltDebugMsg::new(format!("hello  clt #{}", idx).as_bytes()));
            event_clb.on_send(&Default::default(), &clt_msg);
        }
        info!("event_clb: {}", event_clb);
        let found = event_store.find(|_| true, None).await;
        info!("found: {:?}", found);
        let last = event_store.last();
        info!("last: {:?}", last);
        assert_eq!(last, found);
        match found.unwrap().event {
            Event::Send(msg) => assert_eq!(msg, msg),
            _ => panic!("unexpected event"),
        }
    }
}
