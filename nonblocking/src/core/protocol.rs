use crate::prelude::{ConId, ConnectionId, Messenger};
use std::io::Error;

use super::{RecvNonBlocking, SendNonBlocking};

#[allow(unused_variables)]
pub trait Protocol: Messenger + Clone {
    #[inline(always)]
    fn on_connected<C: SendNonBlocking<Self> + RecvNonBlocking<Self> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
        Ok(())
    }

    /// Called immediately before the message is being sent and is a hook to modify the message being sent before it has been sent
    #[inline(always)]
    fn on_send(&self, con_id: &ConId, msg: &mut <Self as Messenger>::SendT) {}

    /// Called after [Self::on_send] and only in the event the attempt to deliver the message resulted in a wouldblock
    /// and will not be retried
    #[inline(always)]
    fn on_wouldblock(&self, con_id: &ConId, msg: &<Self as Messenger>::SendT) {}

    /// Called after [Self::on_send] and only in the event the attempt to deliver the message resulted in an error
    #[inline(always)]
    fn on_error(&self, con_id: &ConId, msg: &<Self as Messenger>::SendT, e: &std::io::Error) {}

    /// Called immediately after the message has been sent, must guarantee that it is only called once per message
    #[inline(always)]
    fn on_sent(&self, con_id: &ConId, msg: &<Self as Messenger>::SendT) {}

    /// Called immediately after the message has been received and and allows to produce a reply with a provided sender
    #[inline(always)]
    fn on_recv<S: SendNonBlocking<Self> + ConnectionId>(&self, msg: &<Self as Messenger>::RecvT, sender: &mut S) -> Result<(), Error> {
        Ok(())
    }
}
