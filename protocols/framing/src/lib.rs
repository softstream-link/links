pub mod prelude;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use log::info;

#[derive(Debug, Clone)]
pub enum ConId {
    Clt(String),
    Svc(String),
}

pub trait Framer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

#[rustfmt::skip]
pub trait Messenger: Debug + Send + Sync + 'static {
    type Message: ByteDeserializeSlice<Self::Message> + ByteSerializeStack + Debug + Send + Sync + 'static;
}


pub trait ProtocolHandler: Messenger + Framer + Send + Sync + 'static {}
// TODO need to add hooks to this trait to handle auto responce

pub trait Callback<MESSENGER: Messenger>:  Debug + Send + Sync + 'static {
    fn on_recv(&mut self, con_id: &ConId, msg: MESSENGER::Message);
    fn on_send(&self, con_id: &ConId, msg: &mut MESSENGER::Message);
}

#[derive(Debug)]
pub struct LoggerCallback<MESSENGER: Messenger> {
    phantom: std::marker::PhantomData<MESSENGER>,
}
impl<MESSENGER: Messenger> LoggerCallback<MESSENGER> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}
// TODO what about session id in the callback?
impl<MESSENGER: Messenger> Callback<MESSENGER> for LoggerCallback<MESSENGER> {
    fn on_recv(&mut self, con_id: &ConId, msg: MESSENGER::Message) {
        info!("LoggerCallback::on_recv {:?} {:?}", con_id, msg);
    }
    fn on_send(&self, con_id: &ConId, msg: &mut MESSENGER::Message) {
        info!("LoggerCallback::on_send {:?} {:?}", con_id, msg);
    }
}