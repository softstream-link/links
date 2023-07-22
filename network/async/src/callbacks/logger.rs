use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use log::{debug, error, info, log_enabled, trace, warn, Level};

use crate::core::{ConId, Messenger};

use super::Callback;

pub type LoggerCallbackRef<MESSENGER> = Arc<LoggerCallback<MESSENGER>>;
#[derive(Debug)]
pub struct LoggerCallback<MESSENGER: Messenger> {
    level: Level,
    p1: std::marker::PhantomData<MESSENGER>,
}
impl<MESSENGER: Messenger> Default for LoggerCallback<MESSENGER> {
    fn default() -> Self {
        Self {
            level: Level::Info,
            p1: std::marker::PhantomData,
        }
    }
}

impl<MESSENGER: Messenger> LoggerCallback<MESSENGER> {
    pub fn new(level: Level) -> Self {
        Self {
            level,
            p1: std::marker::PhantomData,
        }
    }
}

impl<MESSENGER: Messenger> Display for LoggerCallback<MESSENGER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoggerCallback<{}>", self.level)
    }
}

impl<MESSENGER: Messenger> Callback<MESSENGER> for LoggerCallback<MESSENGER> {
    fn on_recv(&self, con_id: &ConId, msg: MESSENGER::Message) {
        if !log_enabled!(self.level) {
            return ();
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
    fn on_send(&self, con_id: &ConId, msg: &MESSENGER::Message) {
        if !log_enabled!(self.level) {
            return ();
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

    use crate::unittest::setup;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;

    use super::*;

    type Log = LoggerCallback<MsgProtocolHandler>;

    #[test]
    fn test_event_log() {
        setup::log::configure();
        let log = Log::default();

        for _ in 0..10 {
            let msg = Msg::Clt(MsgFromClt::new(b"hello".as_slice()));
            log.on_recv(&ConId::default(), msg);
        }
    }
}
