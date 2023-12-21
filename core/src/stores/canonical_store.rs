use std::{
    fmt::{Debug, Display},
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Local};

use crate::{asserted_short_name, core::macros::short_type_name, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalEntry<T: Debug> {
    pub con_id: ConId,
    pub instant: Instant,
    pub time: SystemTime,
    pub msg: Message<T>,
}
impl<T: Debug> CanonicalEntry<T> {
    pub fn try_into_recv(self) -> T {
        match self.msg {
            Message::Recv(t) => t,
            Message::Sent(_) => panic!("{}::try_into_recv: Not {}::Recv variant", asserted_short_name!("Entry", Self), asserted_short_name!("Msg", Message<T>)),
        }
    }
    pub fn try_into_sent(self) -> T {
        match self.msg {
            Message::Recv(_) => panic!("{}::try_into_send: Not a {}::Send variant", asserted_short_name!("Entry", Self), asserted_short_name!("Msg", Message<T>)),
            Message::Sent(t) => t,
        }
    }
}
impl<T: Debug> Display for CanonicalEntry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\t{:?}", self.con_id, self.msg)
    }
}

#[derive(Debug)]
pub struct CanonicalEntryStore<T: Debug + Send + Sync + Clone> {
    store: spin::Mutex<Vec<CanonicalEntry<T>>>,
}
impl<T: Debug + Send + Sync + Clone> CanonicalEntryStore<T> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn lock(&self) -> spin::MutexGuard<'_, Vec<CanonicalEntry<T>>> {
        // let grd = self.store.lock().expect("Could Not Lock MsgStore");
        self.store.lock()
    }
    pub fn push(&self, e: CanonicalEntry<T>) {
        self.lock().push(e);
    }
    pub fn find<P: Fn(&CanonicalEntry<T>) -> bool>(&self, con_id_name: &str, predicate: P, timeout: Option<Duration>) -> Option<CanonicalEntry<T>> {
        let now = Instant::now();
        let timeout = timeout.unwrap_or_else(|| Duration::from_secs(0));
        loop {
            {
                // this scope will drop the lock before the next await
                let events = self.lock();
                let opt = events.iter().rev().find(|entry| entry.con_id.name() == con_id_name && predicate(*entry));
                if opt.is_some() {
                    return opt.cloned(); // because the result is behind a mutex must clone in order to return
                }
            }

            if now.elapsed() > timeout {
                break;
            }
        }
        None
    }
    pub fn find_recv<P: Fn(&T) -> bool>(&self, con_id_name: &str, predicate: P, timeout: Option<Duration>) -> Option<T> {
        let entry = self.find(
            con_id_name,
            |entry| match entry.msg {
                Message::Recv(ref t) => match predicate(t) {
                    true => true,
                    false => false,
                },
                _ => false,
            },
            timeout,
        );
        match entry {
            Some(CanonicalEntry { msg: Message::Recv(t), .. }) => Some(t),
            _ => None,
        }
    }
    pub fn find_sent<P: Fn(&T) -> bool>(&self, con_id_name: &str, predicate: P, timeout: Option<Duration>) -> Option<T> {
        let entry = self.find(
            con_id_name,
            |entry| match entry.msg {
                Message::Sent(ref t) => match predicate(t) {
                    true => true,
                    false => false,
                },
                _ => false,
            },
            timeout,
        );
        match entry {
            Some(CanonicalEntry { msg: Message::Sent(t), .. }) => Some(t),
            _ => None,
        }
    }
    pub fn last(&self) -> Option<CanonicalEntry<T>> {
        self.lock().last().cloned()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }
}
impl<T: Debug + Send + Sync + Clone> Storage<T> for CanonicalEntryStore<T> {
    #[inline(always)]
    fn on_msg(&self, con_id: ConId, msg: Message<T>) {
        self.push(CanonicalEntry {
            con_id,
            instant: Instant::now(),
            time: SystemTime::now(),
            msg,
        })
    }
}
impl<T: Debug + Send + Sync + Clone> Default for CanonicalEntryStore<T> {
    fn default() -> Self {
        Self { store: Default::default() }
    }
}
impl<T: Debug + Send + Sync + Clone> Display for CanonicalEntryStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn writeln<T: Debug>(f: &mut std::fmt::Formatter<'_>, count: usize, delta_window: Duration, entry: &CanonicalEntry<T>) -> std::fmt::Result {
            let dt: DateTime<Local> = entry.time.into();
            let dt = &dt.format("%T.%f").to_string()[..15];
            writeln!(f, "{:<04} Δ{: >15?} {} {}", count, delta_window, dt, entry)?;
            Ok(())
        }
        let msgs = self.lock();
        writeln!(f, "{}<{}, {}>", asserted_short_name!("CanonicalEntryStore", Self), short_type_name::<T>(), msgs.len())?;

        if !msgs.is_empty() {
            let entry1 = msgs.first().expect("Could Not Get First Entry");
            writeln(f, 1, Duration::from_secs(0), entry1)?;
        }

        for (idx, pair) in msgs.windows(2).enumerate() {
            writeln(f, idx + 2, pair[1].instant - pair[0].instant, &pair[1])?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use log::info;

    use crate::unittest::setup::{
        self,
        messenger::{CltTestMessenger, SvcTestMessenger},
        model::*,
    };

    use super::*;

    #[test]
    fn test_callback() {
        setup::log::configure();

        // let event_store_async = EventStoreAsync::new_ref();
        let store = CanonicalEntryStore::<UniTestMsg>::new_ref();
        let clt_clb = StoreCallback::<CltTestMessenger, _, _>::new_ref(store.clone());
        let svc_clb = StoreCallback::<SvcTestMessenger, _, _>::new_ref(store.clone());
        info!("clt_clb: {}", clt_clb);
        info!("svc_clb: {}", svc_clb);

        let svc_on_recv_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"SVC: on_recv Message"));
        let svc_on_sent_msg = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"SVC: on_send Message"));
        svc_clb.on_recv(&ConId::svc(Some("svc"), "0.0.0.0:0", None), &svc_on_recv_msg);
        svc_clb.on_sent(&ConId::svc(Some("svc"), "0.0.0.0:0", None), &svc_on_sent_msg);

        let clt_on_recv_msg = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"CLT: on_recv Message"));
        let clt_on_send_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"CLT: on_send Message"));
        clt_clb.on_recv(&ConId::clt(Some("clt"), None, "0.0.0.0:0"), &clt_on_recv_msg);
        clt_clb.on_sent(&ConId::clt(Some("clt"), None, "0.0.0.0:0"), &clt_on_send_msg);

        info!("store: {}", store);

        // // Entry find
        let last_svc = store.find("svc", |_| true, None).unwrap();
        info!("last_svc: {:?}", last_svc);
        assert_eq!(last_svc.msg, Message::Sent(UniTestMsg::Svc(svc_on_sent_msg.clone())));

        let last_clt = store.find("clt", |_| true, None).unwrap();
        info!("last_clt: {:?}", last_clt);
        assert_eq!(last_clt.msg, Message::Sent(UniTestMsg::Clt(clt_on_send_msg.clone())));

        let last_entry = store.last().unwrap();
        info!("last_entry: {:?}", last_entry);
        assert_eq!(last_entry, last_clt);

        // RECV find unpacks the event into TestMsg
        let svc_recv = store.find_recv("svc", |msg| matches!(msg, UniTestMsg::Clt(CltTestMsg::Dbg(CltTestMsgDebug{text, ..})) if text == &b"SVC: on_recv Message".as_slice().into() ), None);
        info!("svc_recv: {:?}", svc_recv);
        assert_eq!(svc_recv.unwrap(), UniTestMsg::Clt(svc_on_recv_msg));

        // SEND find unpacks the event into TestMsg
        let svc_sent = store.find_sent("svc", |msg| matches!(msg, UniTestMsg::Svc(SvcTestMsg::Dbg(SvcTestMsgDebug{text, ..})) if text == &b"SVC: on_send Message".as_slice().into() ), None);
        info!("svc_sent: {:?}", svc_sent);
        assert_eq!(svc_sent.unwrap(), UniTestMsg::Svc(svc_on_sent_msg));

        // RECV find unpacks the event into TestMsg
        let clt_recv = store.find_recv("clt", |msg| matches!(msg, UniTestMsg::Svc(SvcTestMsg::Dbg(SvcTestMsgDebug{text, ..})) if text == &b"CLT: on_recv Message".as_slice().into() ), None);
        info!("clt_recv: {:?}", clt_recv);
        assert_eq!(clt_recv.unwrap(), UniTestMsg::Svc(clt_on_recv_msg));

        // SEND find unpacks the event into TestMsg
        let clt_sent = store.find_sent("clt", |msg| matches!(msg, UniTestMsg::Clt(CltTestMsg::Dbg(CltTestMsgDebug{text, ..})) if text == &b"CLT: on_send Message".as_slice().into() ), None);
        info!("clt_sent: {:?}", clt_sent);
        assert_eq!(clt_sent.unwrap(), UniTestMsg::Clt(clt_on_send_msg));

        // NOT found
        let not_found = store.find("not_existent", |_| true, None);
        info!("not_found: {:?}", not_found);
        assert_eq!(not_found, None);
    }
}
