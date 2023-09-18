use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::MessengerOld;
use crate::prelude::*;

use super::CallbackSendRecvOld;

pub type Chain<M> = Vec<Arc<dyn CallbackSendRecvOld<M>>>;

#[derive(Debug)]
pub struct ChainCallback<M: MessengerOld> {
    chain: Chain<M>,
}

impl<M: MessengerOld> ChainCallback<M> {
    pub fn new(chain: Chain<M>) -> Self {
        Self { chain }
    }
    pub fn new_ref(chain: Chain<M>) -> Arc<Self> {
        Arc::new(Self::new(chain))
    }
}
impl<M: MessengerOld> Display for ChainCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainCallback<{}>", self.chain.len())
    }
}
impl<M: MessengerOld> CallbackSendRecvOld<M> for ChainCallback<M> {
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
    use links_testing::unittest::setup::model::*;
    use crate::unittest::setup::messenger::TestCltMsgProtocol;
    use links_testing::unittest::setup;
    // use log::info;
    use log::Level;
    #[test]
    fn test_callback() {
        setup::log::configure();
        // let store = EventStoreAsync::new_ref();

        let clbk = ChainCallback::new(vec![
            LoggerCallbackOld::<TestCltMsgProtocol>::new_ref(Level::Info, Level::Info),
            // EventStoreCallback::<TestMsg, TestCltMsgProtocol>::new_ref(store.clone()),
        ]);

        for _ in 0..2 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            clbk.on_send(&ConId::default(), &msg);
        }
        // info!("store: {}", store);
        // assert_eq!(store.len(), 2);
    }
}
