use std::{
    fmt::{Debug, Display},
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        Arc,
    },
};

use crate::{asserted_short_name, fmt_num, prelude::*};

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
    #[inline(always)]
    pub fn sent_count(&self) -> usize {
        self.sent.load(Relaxed)
    }
    #[inline(always)]
    pub fn recv_count(&self) -> usize {
        self.recv.load(Relaxed)
    }
}

impl<M: Messenger> Display for CounterCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<sent: {}, recv: {}>", asserted_short_name!("CounterCallback", Self), fmt_num!(self.sent.load(Relaxed)), fmt_num!(self.recv.load(Relaxed)),)
    }
}

impl<M: Messenger> CallbackRecvSend<M> for CounterCallback<M> {}
#[allow(unused_variables)]
impl<M: Messenger> CallbackRecv<M> for CounterCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: &M::RecvT) {
        self.recv.fetch_add(1, Relaxed);
    }
}
#[allow(unused_variables)]
impl<M: Messenger> CallbackSend<M> for CounterCallback<M> {
    fn on_sent(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {
        self.sent.fetch_add(1, Relaxed);
    }
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use crate::prelude::*;
    use crate::unittest::setup::{self, messenger_old::CltTestMessenger, model::*};
    use log::info;

    #[test]
    fn test_callback() {
        setup::log::configure();
        let clbk = CounterCallback::<CltTestMessenger>::default();
        const N: usize = 1_000;
        for _ in 0..N {
            let msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"hello".as_slice()));
            clbk.on_sent(&ConId::default(), &msg);
        }
        for _ in 0..N {
            let msg = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"hello".as_slice()));
            clbk.on_recv(&ConId::default(), &msg);
        }
        info!("clbk: {}", clbk);
        assert_eq!(N, clbk.sent_count());
        assert_eq!(N, clbk.recv_count());
    }
}
