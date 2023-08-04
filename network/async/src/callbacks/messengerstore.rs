use std::{
    any::type_name,
    fmt::Display,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::CallbackSendRecv;

#[derive(Debug, Clone, PartialEq)]
pub enum MessengerEvent<M: Messenger> {
    Recv(M::RecvMsg),
    Send(M::SendMsg),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessengerEntry<M: Messenger> {
    pub con_id: ConId,
    pub instant: Instant,
    pub payload: MessengerEvent<M>,
}
impl<M: Messenger> MessengerEntry<M> {
    pub fn try_into_recv(&self) -> Result<&M::RecvMsg, &str> {
        match &self.payload {
            MessengerEvent::Recv(msg) => Ok(msg),
            _ => Err("Entry's event is not Recv"),
        }
    }
    pub fn try_into_sent(&self) -> Result<&M::SendMsg, &str> {
        match &self.payload {
            MessengerEvent::Send(msg) => Ok(msg),
            _ => Err("Entry's event is not Send"),
        }
    }
}

impl<M: Messenger> Display for MessengerEntry<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.con_id, self.payload)
    }
}

pub type MessengerStoreCallbackRef<M> = Arc<MessengerStoreCallback<M>>;
#[derive(Debug)]
pub struct MessengerStoreCallback<M: Messenger> {
    store: Mutex<Vec<MessengerEntry<M>>>,
}

impl<M: Messenger> Display for MessengerStoreCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        let events = self.lock();
        writeln!(f, "EventLogCallback<{}, {}>", name, events.len())?;

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

impl<M: Messenger> Default for MessengerStoreCallback<M> {
    fn default() -> Self {
        Self {
            store: Mutex::new(vec![]),
        }
    }
}
impl<M: Messenger> CallbackSendRecv<M> for MessengerStoreCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: <M as Messenger>::RecvMsg) {
        let entry = MessengerEntry {
            con_id: con_id.clone(),
            instant: Instant::now(),
            payload: MessengerEvent::Recv(msg),
        };
        self.push(entry);
    }
    fn on_send(&self, con_id: &ConId, msg: &<M as Messenger>::SendMsg) {
        let entry = MessengerEntry {
            con_id: con_id.clone(),
            instant: Instant::now(),
            payload: MessengerEvent::Send(msg.clone()),
        };
        self.push(entry);
    }
}

impl<M: Messenger> MessengerStoreCallback<M> {
    fn lock(&self) -> MutexGuard<'_, Vec<MessengerEntry<M>>> {
        self.store.lock().expect("Could Not Lock EventLog")
    }
    pub fn push(&self, e: MessengerEntry<M>) {
        let mut events = self.lock();
        events.push(e);
    }
    pub async fn find<P>(
        &self,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<MessengerEntry<M>>
    where
        P: FnMut(&MessengerEntry<M>) -> bool,
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
    pub fn last(&self) -> Option<MessengerEntry<M>> {
        let events = self.lock();
        events.last().cloned()
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    use super::*;

    type EventLog = MessengerStoreCallback<CltMsgProtocol>;

    #[tokio::test]
    async fn test_event_log() {
        setup::log::configure();
        let event_log = EventLog::default();

        #[allow(unused_assignments)]
        let mut clt_msg = CltMsg::Dbg(CltMsgDebug::new(format!("initialized").as_bytes()));
        for idx in 0..10 {
            let svc_msg = SvcMsg::Dbg(SvcMsgDebug::new("hello".as_bytes()));
            event_log.on_recv(&Default::default(), svc_msg.clone());

            clt_msg = CltMsg::Dbg(CltMsgDebug::new(format!("hello  #{}", idx).as_bytes()));
            event_log.on_send(&Default::default(), &clt_msg);
        }
        info!("event_log: {}", event_log);
        let found = event_log.find(|_| true, None).await;
        info!("found: {:?}", found);
        let last = event_log.last();
        info!("last: {:?}", last);
        assert_eq!(last, found);
        match found.unwrap().payload {
            MessengerEvent::Send(msg) => assert_eq!(msg, msg),
            _ => panic!("unexpected event"),
        }
    }
}
