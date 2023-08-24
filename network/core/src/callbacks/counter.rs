use std::{
    fmt::{Debug, Display},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::core::Messenger;
use crate::prelude::*;
use num_format::{Locale, ToFormattedString};

use super::CallbackSendRecv;

#[derive(Debug)]
pub struct CounterCallback<M: Messenger> {
    sent: AtomicUsize,
    recv: AtomicUsize,
    p1: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for CounterCallback<M> {
    fn default() -> Self {
        Self {
            sent: Default::default(),
            recv: Default::default(),
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: Messenger> CounterCallback<M> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: Messenger> Display for CounterCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CounterCallback<sent: {}, recv: {}>",
            self.sent
                .load(Ordering::SeqCst)
                .to_formatted_string(&Locale::en),
            self.recv
                .load(Ordering::SeqCst)
                .to_formatted_string(&Locale::en)
        )
    }
}

impl<M: Messenger> CallbackSendRecv<M> for CounterCallback<M> {
    fn on_recv(&self, _con_id: &ConId, _msg: M::RecvT) {
        self.recv.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    fn on_send(&self, _con_id: &ConId, _msg: &M::SendT) {
        self.sent.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {

    use crate::unittest::setup::messenger::TestCltMsgProtocol;
    use links_testing::unittest::setup;
    use links_testing::unittest::setup::model::*;

    use log::info;

    use super::*;

    #[test]
    fn test_callback() {
        setup::log::configure();
        let clbk = CounterCallback::<TestCltMsgProtocol>::default();

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