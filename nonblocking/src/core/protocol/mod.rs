pub mod persistance;
pub mod state;

use super::{RecvNonBlocking, SendNonBlocking, ReSendNonBlocking, SendStatus};
use crate::prelude::{short_instance_type_name, ConnectionId, Messenger};
use log::{log_enabled, warn};
use std::{io::Error, time::Duration};

/// Core protocol features that will works with any instantiation of [crate::prelude::Clt] and [crate::prelude::Svc] including
/// [crate::prelude::CltRecver], [crate::prelude::CltRecverRef], [crate::prelude::CltSender], [crate::prelude::CltSenderRef]
#[allow(unused_variables)]
pub trait ProtocolCore: Messenger + Sized {
    /// Called immediately after the connection is established and allows user space to perform a connection handshake
    #[inline(always)]
    fn on_connect<C: SendNonBlocking<<Self as Messenger>::SendT> + ReSendNonBlocking<<Self as Messenger>::SendT> + RecvNonBlocking<<Self as Messenger>::RecvT> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
        Ok(())
    }

    /// Called right before the sender is dropped and allows user space to send a message to the peer
    #[inline(always)]
    fn on_disconnect(&self) -> Option<(Duration, <Self as Messenger>::SendT)> {
        None
    }

    /// This is a hook to provide user space ability to perform a logical check and determine if the connection is still valid
    ///
    /// # Warning
    /// Default implementation panics
    #[inline(always)]
    fn is_connected(&self) -> bool {
        if log_enabled!(log::Level::Warn) {
            warn!(
                "NOTE: this is default {}::is_connected implementation which always yields 'false', you should override this method to provide a meaningful implementation.",
                short_instance_type_name(self)
            );
        }
        false
    }

    /// This is a hook to provide user space ability to modify the message right before it is serialized and sent
    #[inline(always)]
    fn on_send<I: ConnectionId>(&self, who: &I, msg: &mut <Self as Messenger>::SendT) {}

    /// Called after [ProtocolCore::on_send] and only in the event the attempt to deliver the message resulted in a wouldblock
    /// and will not be retried
    #[inline(always)]
    fn on_wouldblock<I: ConnectionId>(&self, who: &I, msg: &<Self as Messenger>::SendT) {}

    /// Called after [ProtocolCore::on_send] and only in the event the attempt to deliver the message resulted in an error
    #[inline(always)]
    fn on_error<I: ConnectionId>(&self, who: &I, msg: &<Self as Messenger>::SendT, e: &std::io::Error) {}

    /// Called after [ProtocolCore::on_send] and only in the event the attempt to deliver succeeded. Implementation must guarantee that it is only called once per message actually sent
    #[inline(always)]
    fn on_sent<I: ConnectionId>(&self, who: &I, msg: &<Self as Messenger>::SendT) {}

    /// Called immediately after the message has been received
    #[inline(always)]
    fn on_recv<I: ConnectionId>(&self, who: &I, msg: &<Self as Messenger>::RecvT) {}
}

/// Full set of protocol features that will only work with Ref instances of [crate::prelude::Clt] and [crate::prelude::Svc]
/// which includes [crate::prelude::CltRecverRef], [crate::prelude::CltSenderRef]
///
/// # Important
/// [Clone] implementation of structure implementing [Protocol] must provide a `CLEAN SLATE` state instance,
/// meaning any state captured by the [Protocol] methods must be erased. This is due to the fact that
/// every new connection accepted by [crate::prelude::SvcAcceptor] will get a clone copy of the [Protocol] instance
/// and each connection must maintain its own state.
#[allow(unused_variables)]
pub trait Protocol: ProtocolCore + Clone {
    /// This is a hook to provide user space ability to perform scripted responses, example automatically emulate certain behavior . Called immediately after [ProtocolCore::on_recv].
    #[inline(always)]
    fn send_reply<S: SendNonBlocking<<Self as Messenger>::SendT> + ConnectionId>(&self, msg: &<Self as Messenger>::RecvT, sender: &mut S) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn conf_heart_beat_interval(&self) -> Option<Duration> {
        None
    }
    #[inline(always)]
    fn send_heart_beat<S: SendNonBlocking<<Self as Messenger>::SendT> + ConnectionId>(&self, sender: &mut S) -> Result<SendStatus, Error> {
        Ok(SendStatus::Completed)
    }
}
