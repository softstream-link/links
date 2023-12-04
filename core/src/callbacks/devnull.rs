use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::{asserted_short_name, prelude::*};

#[derive(Debug, Clone)]
pub struct DevNullCallback<M: Messenger> {
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for DevNullCallback<M> {
    fn default() -> Self {
        Self { phantom: std::marker::PhantomData }
    }
}

impl<M: Messenger> DevNullCallback<M> {
    pub fn new_ref() -> Arc<Self> {
        Self::default().into()
    }
}

impl<M: Messenger> Display for DevNullCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", asserted_short_name!("DevNullCallback", Self))
    }
}
impl<M: Messenger> CallbackRecvSend<M> for DevNullCallback<M> {}
impl<M: Messenger> CallbackRecv<M> for DevNullCallback<M> {
    #[allow(unused_variables)]
    #[inline(always)]
    fn on_recv(&self, con_id: &ConId, msg: &<M as Messenger>::RecvT) {}
}
impl<M: Messenger> CallbackSend<M> for DevNullCallback<M> {
    #[allow(unused_variables)]
    #[inline(always)]
    fn on_sent(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {}
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use crate::prelude::*;
    use crate::unittest::setup::{self, messenger::CltTestMessenger, model::*};

    #[test]
    fn test_callback() {
        setup::log::configure_level(log::LevelFilter::Trace);
        let clbk = DevNullCallback::<CltTestMessenger>::new_ref();

        for _ in 0..2 {
            let msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"hello".as_slice()));
            clbk.on_sent(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"hello".as_slice()));
            clbk.on_recv(&ConId::default(), &msg);
        }
    }
}
