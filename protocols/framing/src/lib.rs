pub mod prelude;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use log::info;

pub trait Framer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

#[rustfmt::skip]
pub trait Messenger: Send + Sync + 'static {
    type Message: ByteDeserializeSlice<Self::Message> + ByteSerializeStack + Debug + Send + Sync + 'static;
}


pub trait ProtocolHandler: Messenger + Framer + Send + Sync + 'static {}
// TODO need to add hooks to this trait to handle auto responce

pub trait Callback<MESSENGER: Messenger>:  Send + Sync + 'static {
    fn on_recv(&mut self, msg: MESSENGER::Message);
    fn on_send(&self, msg: &mut MESSENGER::Message);
}

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
    fn on_recv(&mut self, msg: MESSENGER::Message) {
        info!("LoggerCallback::on_recv {:?}", msg);
    }
    fn on_send(&self, msg: &mut MESSENGER::Message) {
        info!("LoggerCallback::on_send {:?}", msg)
    }
}