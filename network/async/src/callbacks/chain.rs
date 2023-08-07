use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::{ConId, Messenger};

use super::CallbackSendRecv;

pub type Chain<M> = Vec<Arc<dyn CallbackSendRecv<M>>>;

#[derive(Debug)]
pub struct ChainCallback<M: Messenger> {
    chain: Chain<M>,
}

impl<M: Messenger> ChainCallback<M> {
    pub fn new(chain: Chain<M>) -> Self {
        Self { chain }
    }
    pub fn new_ref(chain: Chain<M>) -> Arc<Self> {
        Arc::new(Self::new(chain))
    }
}
impl<M: Messenger> Display for ChainCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainCallback<{}>", self.chain.len())
    }
}
impl<M: Messenger> CallbackSendRecv<M> for ChainCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT) {
        for callback in self.chain.iter() {
            callback.on_recv(con_id, msg.clone());
        }
    }
    fn on_send(&self, con_id: &ConId, msg: &M::SendT) {
        for callback in self.chain.iter() {
            callback.on_send(con_id, msg);
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::callbacks::eventstore::EventStore;
    use crate::prelude::*;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;
    use log::info;
    use log::Level;
    #[test]
    fn test_event_log() {
        setup::log::configure();
        let store = EventStore::new_ref();

        let callback = ChainCallback::new(vec![
            LoggerCallback::new_ref(Level::Info, Level::Info),
            EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new_ref(store.clone()),
        ]);

        for _ in 0..2 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            callback.on_send(&ConId::default(), &msg);
        }
        info!("store: {}", store);
        assert_eq!(store.len(), 2);
    }
}
