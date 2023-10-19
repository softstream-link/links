use std::{
    fmt::{Debug, Display},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::prelude::*;
use crate::{core::MessengerOld, fmt_num};

use super::CallbackSendRecvOld;

#[derive(Debug)]
pub struct CounterCallbackOld<M: MessengerOld> {
    sent: AtomicUsize,
    recv: AtomicUsize,
    p1: std::marker::PhantomData<M>,
}
impl<M: MessengerOld> Default for CounterCallbackOld<M> {
    fn default() -> Self {
        Self {
            sent: Default::default(),
            recv: Default::default(),
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: MessengerOld> CounterCallbackOld<M> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: MessengerOld> Display for CounterCallbackOld<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CounterCallback<sent: {}, recv: {}>",
            fmt_num!(self.sent.load(Ordering::SeqCst)),
            fmt_num!(self.recv.load(Ordering::SeqCst))
        )
    }
}

impl<M: MessengerOld> CallbackSendRecvOld<M> for CounterCallbackOld<M> {
    fn on_recv(&self, _con_id: &ConId, _msg: M::RecvT) {
        self.recv.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn on_send(&self, _con_id: &ConId, _msg: &M::SendT) {
        self.sent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {

    use crate::unittest::setup;
    use crate::unittest::setup::messenger_old::CltTestMessenger;
    use crate::unittest::setup::model::*;

    use log::info;

    use super::*;

    #[test]
    fn test_callback() {
        setup::log::configure();
        let clbk = CounterCallbackOld::<CltTestMessenger>::default();

        for _ in 0..1000 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            clbk.on_send(&ConId::default(), &msg);
        }
        for _ in 0..1000 {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
            clbk.on_recv(&ConId::default(), msg);
        }
        info!("clbk: {}", clbk);
    }
}
