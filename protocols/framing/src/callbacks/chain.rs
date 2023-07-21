use std::{fmt::Debug, sync::Arc};

use crate::{ConId, Messenger};

use super::Callback;

pub type ChainCallbackRef<MESSENGER> = Arc<ChainCallback<MESSENGER>>;
pub type Chain<MESSENGER> = Vec<Arc<dyn Callback<MESSENGER>>>;
#[derive(Debug)]
pub struct ChainCallback<MESSENGER: Messenger> {
    chain: Chain<MESSENGER>,
}

impl<MESSENGER: Messenger> ChainCallback<MESSENGER> {
    pub fn new(chain: Chain<MESSENGER>) -> Self {
        Self { chain }
    }
}

impl<MESSENGER: Messenger> Callback<MESSENGER> for ChainCallback<MESSENGER> {
    fn on_recv(&self, con_id: &ConId, msg: MESSENGER::Message) {
        for callback in self.chain.iter() {
            callback.on_recv(con_id, msg.clone());
        }
    }
    fn on_send(&self, con_id: &ConId, msg: &MESSENGER::Message) {
        for callback in self.chain.iter() {
            callback.on_send(con_id, msg);
        }
    }
}

#[cfg(test)]
mod test {

    use crate::callbacks::eventlog::EventLogCallbackRef;
    use crate::callbacks::logger::LoggerCallbackRef;
    use crate::unittest::setup;
    use crate::unittest::setup::callbacks::*;

    use super::*;

    type EventLog = EventLogCallbackRef<MessengerImpl>;
    type Logger = LoggerCallbackRef<MessengerImpl>;

    #[test]
    fn test_event_log() {
        setup::log::configure();
        let chain: Chain<MessengerImpl> = vec![EventLog::default(), Logger::default()];
        let callback = ChainCallback::new(chain);

        for _ in 0..10 {
            let msg = PayLoad::new(b"hello".as_slice());
            callback.on_recv(&ConId::default(), msg);
        }
    }
}
