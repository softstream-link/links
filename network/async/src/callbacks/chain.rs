use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::{ConId, Messenger};

use super::CallbackSendRecv;

pub type ChainCallbackRef<M> = Arc<ChainCallback<M>>;
pub type Chain<M> = Vec<Arc<dyn CallbackSendRecv<M>>>;
#[derive(Debug)]
pub struct ChainCallback<M: Messenger> {
    chain: Chain<M>,
}

impl<M: Messenger> ChainCallback<M> {
    pub fn new(chain: Chain<M>) -> Self {
        Self { chain }
    }
    pub fn new_ref(chain: Chain<M>) -> ChainCallbackRef<M> {
        Arc::new(Self::new(chain))
    }
}
impl<M: Messenger> Display for ChainCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainCallback<{}>", self.chain.len())
    }
}
impl<M: Messenger> CallbackSendRecv<M> for ChainCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvMsg) {
        for callback in self.chain.iter() {
            callback.on_recv(con_id, msg.clone());
        }
    }
    fn on_send(&self, con_id: &ConId, msg: &M::SendMsg) {
        for callback in self.chain.iter() {
            callback.on_send(con_id, msg);
        }
    }
}

#[cfg(test)]
mod test {

    use crate::callbacks::messengerstore::MessengerStoreCallbackRef;
    use crate::callbacks::logger::LoggerCallbackRef;
    use links_testing::unittest::setup;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;

    use super::*;

    type EventLog = MessengerStoreCallbackRef<CltMsgProtocol>;
    type Logger = LoggerCallbackRef<CltMsgProtocol>;

    #[test]
    fn test_event_log() {
        setup::log::configure();
        let chain: Chain<CltMsgProtocol> = vec![EventLog::default(), Logger::default()];
        let callback = ChainCallback::new(chain);

        for _ in 0..10 {
            let msg = CltMsg::Dbg(CltMsgDebug::new(b"hello".as_slice()));
            callback.on_send(&ConId::default(), &msg);
        }
    }
}
