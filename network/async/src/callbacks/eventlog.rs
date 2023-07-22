use std::{
    fmt::Display,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use tokio::task::yield_now;

use crate::core::{ConId, Messenger};

use super::Callback;

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Send,
    Recv,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry<MESSENGER: Messenger> {
    pub con_id: ConId,
    pub direction: Direction,
    pub instant: Instant,
    pub msg: MESSENGER::Message,
}
impl<MESSENGER: Messenger> Display for Entry<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?}",
            self.con_id,
            self.direction,
            // self.instant,
            self.msg
        )
    }
}

impl<MESSENGER: Messenger> From<(ConId, MESSENGER::Message)> for Entry<MESSENGER> {
    fn from(value: (ConId, MESSENGER::Message)) -> Self {
        let (con_id, msg) = value;
        Self {
            con_id,
            direction: Direction::Recv,
            instant: Instant::now(),
            msg,
        }
    }
}

pub type EventLogCallbackRef<MESSENGER> = Arc<EventLogCallback<MESSENGER>>;
#[derive(Debug)]
pub struct EventLogCallback<MESSENGER: Messenger> {
    events: Mutex<Vec<Entry<MESSENGER>>>,
}

impl<MESSENGER: Messenger> Display for EventLogCallback<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let events = self.lock();
        writeln!(f, "EventLogCallback len: {}", events.len())?;

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
    fn on_recv(&self, con_id: &ConId, msg: <MESSENGER as Messenger>::Message) {
        let entry = Entry::from((con_id.clone(), msg));
        self.push(entry);
    }
    fn on_send(&self, con_id: &ConId, msg: &<MESSENGER as Messenger>::Message) {
        let entry = Entry {
            con_id: con_id.clone(),
            direction: Direction::Send,
            instant: Instant::now(),
            msg: msg.clone(),
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

    type EventLog = EventLogCallback<MsgProtocolHandler>;

    #[tokio::test]
    async fn test_event_log() {
        setup::log::configure();
        let event_log = EventLog::default();

        let mut msg = Msg::Clt(MsgFromClt::new(format!("hello").as_bytes()));
        for idx in 0..10 {
            msg = Msg::Clt(MsgFromClt::new(format!("hello  #{}", idx).as_bytes()));
            let entry = Entry::from((ConId::default(), msg.clone()));
            event_log.push(entry);
        }
        info!("event_log: {}", event_log);
        let found = event_log.find(|_| true, None).await;
        info!("found: {:?}", found);
        let last = event_log.last();
        info!("last: {:?}", last);
        assert_eq!(last, found);
        assert_eq!(found.unwrap().msg, msg);
    }
}
