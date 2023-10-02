use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use log::{debug, error, info, log_enabled, trace, warn, Level};

use crate::prelude::*;

use super::CallbackRecvSend;

#[derive(Debug, Clone)]
pub struct LoggerCallback<M: Messenger> {
    level_recv: Level,
    level_send: Level,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for LoggerCallback<M> {
    fn default() -> Self {
        Self {
            level_recv: Level::Info,
            level_send: Level::Info,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<M: Messenger> LoggerCallback<M> {
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

impl<M: Messenger> Display for LoggerCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoggerCallback<recv: {}, send: {}>",
            self.level_recv, self.level_send
        )
    }
}

impl<M: Messenger> CallbackRecvSend<M> for LoggerCallback<M> {}
impl<M: Messenger> CallbackRecv<M> for LoggerCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: &<M as Messenger>::RecvT) {
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
impl<M: Messenger> CallbackSend<M> for LoggerCallback<M> {
    fn on_sent(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {
        if !log_enabled!(self.level_send) {
            return;
        }
        let text = format!("LoggerCallback::on_sent {} {:?}", con_id, msg);
        match self.level_send {
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

//     use crate::unittest::setup::messenger::TestCltMessenger;
//     use links_network_core::unittest::setup;
//     use links_testing::unittest::setup::model::*;

//     use super::*;

//     #[test]
//     fn test_callback() {
//         setup::log::configure_level(log::LevelFilter::Trace);
//         let clbk = LoggerCallbackRecv::<TestCltMessenger>::with_level(Level::Trace, Level::Trace);

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
