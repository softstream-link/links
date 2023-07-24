use std::{
    any::type_name,
    fmt::Display,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::Callback;

#[derive(Debug, Clone, PartialEq)]
pub enum Event<MESSENGER: Messenger> {
    Recv(MESSENGER::RecvMsg),
    Send(MESSENGER::SendMsg),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<MESSENGER: Messenger> {
    pub con_id: ConId,
    pub instant: Instant,
    pub event: Event<MESSENGER>,
}
impl<MESSENGER: Messenger> Entry<MESSENGER> {
    pub fn try_into_recv(&self) -> Result<&MESSENGER::RecvMsg, &str> {
        match &self.event {
            Event::Recv(msg) => Ok(msg),
            _ => Err("Entry's event is not Recv"),
        }
    }
    pub fn try_into_sent(&self) -> Result<&MESSENGER::SendMsg, &str> {
        match &self.event {
            Event::Send(msg) => Ok(msg),
            _ => Err("Entry's event is not Send"),
        }
    }
}

impl<MESSENGER: Messenger> Display for Entry<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?}", self.con_id, self.event)
    }
}

pub type EventLogCallbackRef<MESSENGER> = Arc<EventLogCallback<MESSENGER>>;
#[derive(Debug)]
pub struct EventLogCallback<MESSENGER: Messenger> {
    events: Mutex<Vec<Entry<MESSENGER>>>,
}

impl<MESSENGER: Messenger> Display for EventLogCallback<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<MESSENGER>()
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

impl<MESSENGER: Messenger> Default for EventLogCallback<MESSENGER> {
    fn default() -> Self {
        Self {
            events: Mutex::new(vec![]),
        }
    }
}
impl<MESSENGER: Messenger> Callback<MESSENGER> for EventLogCallback<MESSENGER> {
    fn on_recv(&self, con_id: &ConId, msg: <MESSENGER as Messenger>::RecvMsg) {
        let entry = Entry {
            con_id: con_id.clone(),
            instant: Instant::now(),
            event: Event::Recv(msg.clone()),
        };
        self.push(entry);
    }
    fn on_send(&self, con_id: &ConId, msg: &<MESSENGER as Messenger>::SendMsg) {
        let entry = Entry {
            con_id: con_id.clone(),
            instant: Instant::now(),
            event: Event::Send(msg.clone()),
        };
        self.push(entry);
    }
}

impl<MESSENGER: Messenger> EventLogCallback<MESSENGER> {
    fn lock(&self) -> MutexGuard<'_, Vec<Entry<MESSENGER>>> {
        self.events.lock().expect("Could Not Lock EventLog")
    }
    pub fn push(&self, e: Entry<MESSENGER>) {
        let mut events = self.lock();
        events.push(e);
    }
    pub async fn find<P>(
        &self,
        mut predicate: P,
        timeout: Option<Duration>,
    ) -> Option<Entry<MESSENGER>>
    where
        P: FnMut(&Entry<MESSENGER>) -> bool,
    {
        let now = Instant::now();
        let timeout = timeout.unwrap_or_else(|| Duration::from_secs(0));
        loop {
            let events = self.lock();
            let opt = events.iter().rev().find(|entry| predicate(*entry));
            if opt.is_some() {
                return opt.cloned();
            }
            drop(events);

            if now.elapsed() > timeout {
                break;
            }
            yield_now().await;
        }
        None
    }
    pub fn last(&self) -> Option<Entry<MESSENGER>> {
        let events = self.lock();
        events.last().cloned()
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::unittest::setup;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;

    use super::*;

    type EventLog = EventLogCallback<CltMsgProtocol>;

    #[tokio::test]
    async fn test_event_log() {
        setup::log::configure();
        let event_log = EventLog::default();

        #[allow(unused_assignments)]
        let mut msg = CltMsg::new(format!("initialized").as_bytes());
        for idx in 0..10 {
            msg = CltMsg::new(format!("hello  #{}", idx).as_bytes());
            event_log.on_send(&Default::default(), &msg);
        }
        info!("event_log: {}", event_log);
        let found = event_log.find(|_| true, None).await;
        info!("found: {:?}", found);
        let last = event_log.last();
        info!("last: {:?}", last);
        assert_eq!(last, found);
        match found.unwrap().event {
            Event::Send(msg) => assert_eq!(msg, msg),
            _ => panic!("unexpected event"),
        }
    }
}
