use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use log::{debug, error, info, log_enabled, trace, warn, Level};

use crate::core::{conid::ConId, Messenger};

use super::CallbackSendRecv;

#[derive(Debug)]
pub struct LoggerCallback<M: Messenger> {
    level_recv: Level,
    level_send: Level,
    p1: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for LoggerCallback<M> {
    fn default() -> Self {
        Self {
            level_recv: Level::Info,
            level_send: Level::Info,
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: Messenger> LoggerCallback<M> {
    pub fn new(level_recv: Level, level_send: Level) -> Self {
        Self {
            level_recv,
            level_send,
            p1: std::marker::PhantomData,
        }
    }
    pub fn new_ref(level_recv: Level, level_send: Level) -> Arc<Self> {
        Arc::new(Self::new(level_recv, level_send))
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

impl<M: Messenger> CallbackSendRecv<M> for LoggerCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT) {
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
    fn on_send(&self, con_id: &ConId, msg: &M::SendT) {
        if !log_enabled!(self.level_send) {
            return;
        }
        let text = format!("LoggerCallback::on_send {} {:?}", con_id, msg);
        match self.level_send {
            Level::Error => error!("{}", text),
            Level::Warn => warn!("{}", text),
            Level::Info => info!("{}", text),
            Level::Debug => debug!("{}", text),
            Level::Trace => trace!("{}", text),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use links_testing::unittest::setup;

    use super::*;

    #[test]
    fn test_event_log() {
        setup::log::configure_level(log::LevelFilter::Trace);
        let log = LoggerCallback::<TestCltMsgProtocol>::new_ref(Level::Trace, Level::Trace);

        for _ in 0..2 {
            let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
            log.on_send(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
            log.on_recv(&ConId::default(), msg);
        }
    }
}
