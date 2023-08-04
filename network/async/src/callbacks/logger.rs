use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use log::{debug, error, info, log_enabled, trace, warn, Level};

use crate::core::{ConId, Messenger};

use super::CallbackSendRecv;

pub type LoggerCallbackRef<M> = Arc<LoggerCallback<M>>;
#[derive(Debug)]
pub struct LoggerCallback<M: Messenger> {
    level: Level,
    p1: std::marker::PhantomData<M>,
}
impl<M: Messenger> Default for LoggerCallback<M> {
    fn default() -> Self {
        Self {
            level: Level::Info,
            p1: std::marker::PhantomData,
        }
    }
}

impl<M: Messenger> LoggerCallback<M> {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            p1: std::marker::PhantomData,
        }
    }
    pub fn new_ref(level: Level) -> LoggerCallbackRef<M>{
        Arc::new(Self::new(level))
    }
}

impl<M: Messenger> Display for LoggerCallback<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoggerCallback<{}>", self.level)
    }
}

impl<M: Messenger> CallbackSendRecv<M> for LoggerCallback<M> {
    fn on_recv(&self, con_id: &ConId, msg: M::RecvT) {
        if !log_enabled!(self.level) {
            return;
        }
        let text = format!("LoggerCallback::on_recv {} {:?}", con_id, msg);
        match self.level {
            Level::Error => error!("{}", text),
            Level::Warn => warn!("{}", text),
            Level::Info => info!("{}", text),
            Level::Debug => debug!("{}", text),
            Level::Trace => trace!("{}", text),
        }
    }
    fn on_send(&self, con_id: &ConId, msg: &M::SendT) {
        if !log_enabled!(self.level) {
            return;
        }
        let text = format!("LoggerCallback::on_send {} {:?}", con_id, msg);
        match self.level {
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
        setup::log::configure_at(log::LevelFilter::Trace);
        let log = LoggerCallback::<CltMsgProtocol>::new_ref(Level::Trace);

        for _ in 0..2 {
            let msg = CltMsg::Dbg(CltMsgDebug::new(b"hello".as_slice()));
            log.on_send(&ConId::default(), &msg);
        }
        for _ in 0..2 {
            let msg = SvcMsg::Dbg(SvcMsgDebug::new(b"hello".as_slice()));
            log.on_recv(&ConId::default(), msg);
        }
    }
}
