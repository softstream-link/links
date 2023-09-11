use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::prelude::*;

use super::CallbackSendRecvNew;

#[derive(Debug, Clone)]
pub struct DevNullCallbackNew<M: MessengerNew> {
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew> Default for DevNullCallbackNew<M> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<M: MessengerNew> DevNullCallbackNew<M> {
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: MessengerNew> Display for DevNullCallbackNew<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DevNullCallback", 
        )
    }
}
impl<M: MessengerNew> CallbackSendRecvNew<M> for DevNullCallbackNew<M> {}
impl<M: MessengerNew> CallbackRecv<M> for DevNullCallbackNew<M> {
    #[allow(unused_variables)]
    fn on_recv(&self, con_id: &ConId, msg: <M as MessengerNew>::RecvT) {}
}
impl<M: MessengerNew> CallbackSend<M> for DevNullCallbackNew<M> {
    #[allow(unused_variables)]
    fn on_send(&self, con_id: &ConId, msg: &mut <M as MessengerNew>::SendT) {}
}

// #[cfg(test)]
// mod test {

//     use crate::unittest::setup::messenger::TestCltMsgProtocol;
//     use links_testing::unittest::setup;
//     use links_testing::unittest::setup::model::*;

//     use super::*;

//     #[test]
//     fn test_callback() {
//         setup::log::configure_level(log::LevelFilter::Trace);
//         let clbk = DevNullCallbackRecv::<TestCltMsgProtocol>::with_level(Level::Trace, Level::Trace);

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
