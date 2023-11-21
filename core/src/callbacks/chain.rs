use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::{asserted_short_name, prelude::*};

pub type Chain<M> = Vec<Arc<dyn CallbackRecvSend<M>>>;

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
        write!(
            f,
            "{}<{}, [{}]>",
            asserted_short_name!("ChainCallback", Self),
            self.chain.len(),
            self.chain.iter().map(|c| format!("{}", c)).collect::<Vec<_>>().join(", ")
        )
    }
}
impl<M: Messenger> CallbackRecvSend<M> for ChainCallback<M> {}
impl<M: Messenger> CallbackRecv<M> for ChainCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: &<M as Messenger>::RecvT) {
        for callback in self.chain.iter() {
            callback.on_recv(con_id, msg);
        }
    }
}
impl<M: Messenger> CallbackSend<M> for ChainCallback<M> {
    fn on_fail(&self, con_id: &ConId, msg: &<M as Messenger>::SendT, e: &std::io::Error) {
        for callback in self.chain.iter() {
            callback.on_fail(con_id, msg, e);
        }
    }
    fn on_send(&self, con_id: &ConId, msg: &mut <M as Messenger>::SendT) {
        for callback in self.chain.iter() {
            callback.on_send(con_id, msg);
        }
    }
    fn on_sent(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {
        for callback in self.chain.iter() {
            callback.on_sent(con_id, msg);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use crate::prelude::*;
    use crate::unittest::setup::{self, messenger::CltTestMessenger, model::*};
    use log::info;

    #[test]
    fn test_callback() {
        setup::log::configure();
        let counter = CounterCallback::new_ref();

        let clbk = ChainCallback::<CltTestMessenger>::new(vec![LoggerCallback::new_ref(), counter.clone()]);

        for _ in 0..2 {
            let msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"hello".as_slice()));
            clbk.on_sent(&ConId::default(), &msg);
        }
        info!("clbk: {}", clbk);
        assert_eq!(counter.sent_count(), 2);
        assert_eq!(counter.send_count(), 0);
        assert_eq!(counter.fail_count(), 0);
        assert_eq!(counter.recv_count(), 0);
    }
}
