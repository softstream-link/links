use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use log::{debug, error, info, log_enabled, trace, warn, Level};

use crate::prelude::*;

use super::CallbackSendRecvNew;

#[derive(Debug, Clone)]
pub struct LoggerCallbackNew<M: MessengerNew> {
    level_recv: Level,
    level_send: Level,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew> Default for LoggerCallbackNew<M> {
    fn default() -> Self {
        Self {
            level_recv: Level::Info,
            level_send: Level::Info,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<M: MessengerNew> LoggerCallbackNew<M> {
    pub fn with_level(level_recv: Level, level_send: Level) -> Self {
        Self {
            level_recv,
            level_send,
            phantom: std::marker::PhantomData,
        }
    }
    pub fn with_level_ref(level_recv: Level, level_send: Level) -> Arc<Self> {
        Arc::new(Self::with_level(level_recv, level_send))
    }
    pub fn new_ref() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl<M: MessengerNew> Display for LoggerCallbackNew<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoggerCallback<recv: {}, send: {}>",
            self.level_recv, self.level_send
        )
    }
}

impl<M: MessengerNew> CallbackSendRecvNew<M> for LoggerCallbackNew<M> {}
impl<M: MessengerNew> CallbackRecv<M> for LoggerCallbackNew<M> {
    fn on_recv(&self, con_id: &ConId, msg: &<M as MessengerNew>::RecvT) {
        if !log_enabled!(self.level_recv) {
            return;
        }
        let text = format!("LoggerCallback::on_recv {} {:?}", con_id, msg);
        match self.level_recv {
            Level::Error => error!("{}", text),
            Level::Warn => warn!("{}", text),
            Level::Info => info!("{}", text),
            Level::Debug => debug!("{}", text),
            Level::Trace => trace!("{}", text),
        }
    }
}
impl<M: MessengerNew> CallbackSend<M> for LoggerCallbackNew<M> {
    fn on_send(&self, con_id: &ConId, msg: &mut <M as MessengerNew>::SendT) {
        if !log_enabled!(self.level_recv) {
            return;
        }
        let text = format!("LoggerCallback::on_send {} {:?}", con_id, msg);
        match self.level_recv {
            Level::Error => error!("{}", text),
            Level::Warn => warn!("{}", text),
            Level::Info => info!("{}", text),
            Level::Debug => debug!("{}", text),
            Level::Trace => trace!("{}", text),
        }
    }
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
//         let clbk = LoggerCallbackRecv::<TestCltMsgProtocol>::with_level(Level::Trace, Level::Trace);

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