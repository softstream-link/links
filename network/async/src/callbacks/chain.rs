use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::{ConId, Messenger};

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
impl<MESSENGER: Messenger> Display for ChainCallback<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainCallback<{}>", self.chain.len())
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
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;

    use super::*;

    type EventLog = EventLogCallbackRef<MsgProtocolHandler>;
    type Logger = LoggerCallbackRef<MsgProtocolHandler>;

    #[test]
    fn test_event_log() {
        setup::log::configure();
        let chain: Chain<MsgProtocolHandler> = vec![EventLog::default(), Logger::default()];
        let callback = ChainCallback::new(chain);

        for _ in 0..10 {
            let msg = Msg::Clt(MsgFromClt::new(b"hello".as_slice()));
            callback.on_recv(&ConId::default(), msg);
        }
    }
}
