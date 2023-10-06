use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::prelude::*;

use super::CallbackRecvSend;

#[derive(Debug, Clone)]
pub struct DevNullCallback<M: Messenger> {
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for DevNullCallback<M> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
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
        write!(
            f,
            "DevNullCallback", 
        )
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

// #[cfg(test)]
// mod test {

//     use crate::unittest::setup::messenger::CltTestMessenger;
//     use links_core::unittest::setup;
//     use links_testing::unittest::setup::model::*;

//     use super::*;

//     #[test]
//     fn test_callback() {
//         setup::log::configure_level(log::LevelFilter::Trace);
//         let clbk = DevNullCallbackRecv::<CltTestMessenger>::with_level(Level::Trace, Level::Trace);

//         for _ in 0..2 {
//             let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
//             clbk.on_send(&ConId::default(), &msg);
//         }
//         for _ in 0..2 {
//             let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
//             clbk.on_recv(&ConId::default(), msg);
//         }
//     }
// }
