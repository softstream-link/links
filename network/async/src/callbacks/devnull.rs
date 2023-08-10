use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::{conid::ConId, Messenger};

use super::CallbackSendRecv;

#[derive(Debug)]
pub struct DevNullCallback<M: Messenger> {
    p1: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for DevNullCallback<M> {
    fn default() -> Self {
        Self {
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: Messenger> DevNullCallback<M> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: Messenger> Display for DevNullCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DevNullCallback")
    }
}

impl<M: Messenger> CallbackSendRecv<M> for DevNullCallback<M> {
    fn on_recv(&self, _con_id: &ConId, _msg: M::RecvT) {}
    fn on_send(&self, _con_id: &ConId, _msg: &M::SendT) {}
}

#[cfg(test)]
mod test {

    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    use super::*;

    #[test]
    fn test_event_log() {
        setup::log::configure();
        let log = DevNullCallback::<TestCltMsgProtocol>::default();

        for _ in 0..2 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            log.on_send(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
            log.on_recv(&ConId::default(), msg);
        }
    }
}
