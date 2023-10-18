use std::{
    fmt::{Debug, Display},
    sync::Arc,
};


use crate::prelude::*;

use super::CallbackSendRecvOld;

#[derive(Debug)]
pub struct DevNullCallbackOld<M: MessengerOld> {
    p1: std::marker::PhantomData<M>,
}
impl<M: MessengerOld> Default for DevNullCallbackOld<M> {
    fn default() -> Self {
        Self {
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: MessengerOld> DevNullCallbackOld<M> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: MessengerOld> Display for DevNullCallbackOld<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DevNullCallback")
    }
}

impl<M: MessengerOld> CallbackSendRecvOld<M> for DevNullCallbackOld<M> {
    fn on_recv(&self, _con_id: &ConId, _msg: M::RecvT) {}
    fn on_send(&self, _con_id: &ConId, _msg: &M::SendT) {}
}

#[cfg(test)]
mod test {

    use crate::unittest::setup::messenger_old::CltTestMessenger;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup;

    use super::*;

    #[test]
    fn test_callback() {
        setup::log::configure();
        let clbk = DevNullCallbackOld::<CltTestMessenger>::default();

        for _ in 0..2 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            clbk.on_send(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
            clbk.on_recv(&ConId::default(), msg);
        }
    }
}
