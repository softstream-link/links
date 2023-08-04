use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::core::{ConId, Messenger};

use super::CallbackSendRecv;

pub type DevNullCallbackRef<M> = Arc<DevNullCallback<M>>;
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
    pub fn new_ref() -> DevNullCallbackRef<M> {
        Arc::new(Self::default())
    }
}

impl<M: Messenger> Display for DevNullCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DevNullCallback")
    }
}

impl<M: Messenger> CallbackSendRecv<M> for DevNullCallback<M> {
    fn on_recv(&self, _con_id: &ConId, _msg: M::RecvMsg) {}
    fn on_send(&self, _con_id: &ConId, _msg: &M::SendMsg) {}
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
        let log = DevNullCallback::<CltMsgProtocol>::default();

        for _ in 0..2 {
            let msg = CltMsg::Dbg(CltMsgDebug::new(b"hello".as_slice()));
            log.on_send(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = SvcMsg::Dbg(SvcMsgDebug::new(b"hello".as_slice()));
            log.on_recv(&ConId::default(), msg);
        }
    }
}
