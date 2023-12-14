use super::{RecvNonBlocking, SendNonBlocking, SendStatus};
use crate::prelude::{short_instance_type_name, ConnectionId, Messenger};
use log::{log_enabled, warn};
use spin::MutexGuard;
use std::{io::Error, sync::Arc, time::Duration};

/// Core protocol features that will works with any instantiation of [crate::prelude::Clt] and [crate::prelude::Svc] including
/// [crate::prelude::CltRecver], [crate::prelude::CltRecverRef], [crate::prelude::CltSender], [crate::prelude::CltSenderRef]
#[allow(unused_variables)]
pub trait ProtocolCore: Messenger + Sized {
    /// Called immediately after the connection is established and allows user space to perform a connection handshake
    #[inline(always)]
    fn on_connected<C: SendNonBlocking<Self> + RecvNonBlocking<Self> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
        Ok(())
    }

    /// This is a hook to provide user space ability to perform a logical check and determine if the connection is still valid
    ///
    /// # Warning
    /// Default implementation panics
    #[inline(always)]
    fn is_connected(&self) -> bool {
        if log_enabled!(log::Level::Warn) {
            warn!(
                "NOTE: this is default {}::is_connected implementation which always yields 'true', you should override this method to provide a meaningful implementation.",
                short_instance_type_name(self)
            );
        }
        true
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
    fn send_reply<S: SendNonBlocking<Self> + ConnectionId>(&self, msg: &<Self as Messenger>::RecvT, sender: &mut S) -> Result<(), Error> {
        Ok(())
    }

    #[inline(always)]
    fn conf_heart_beat_interval(&self) -> Option<Duration> {
        None
    }
    #[inline(always)]
    fn send_heart_beat<S: SendNonBlocking<Self> + ConnectionId>(&self, sender: &mut S) -> Result<SendStatus, Error> {
        Ok(SendStatus::Completed)
    }
}

/// This facility helps user capture and maintain protocol state.
///
/// # Key Features
/// * It is useful because all [Protocol] methods are called with an immutable reference `&self` so to maintain state, user must use interior mutability.
/// * It is also thread safe with help of [spin::Mutex] since different protocol methods can potentially be invoked from different threads. Ex: [ProtocolCore::on_sent] and [ProtocolCore::on_recv]
/// * It correctly handles [Clone] implementation by cloning state `T` instead of [Arc] clone of the state.
#[derive(Debug)]
pub struct ProtocolState<T: Clone>(Arc<spin::Mutex<T>>);
impl<T: Clone> ProtocolState<T> {
    #[inline(always)]
    pub fn new(state: T) -> Self {
        Self(Arc::new(spin::Mutex::new(state)))
    }
    #[inline(always)]
    pub fn set(&self, state: T) {
        *self.0.lock() = state;
    }
    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock()
    }
}
impl<T: Clone + Default> Default for ProtocolState<T> {
    fn default() -> Self {
        Self(Arc::new(spin::Mutex::new(T::default())))
    }
}
impl<T: Clone> Clone for ProtocolState<T> {
    fn clone(&self) -> Self {
        Self(Arc::new(spin::Mutex::new(self.lock().clone())))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_protocol_state() {
        let state1 = ProtocolState::<Option<usize>>::default();
        assert_eq!(*state1.lock(), None);
        state1.set(Some(1));
        assert_eq!(*state1.lock(), Some(1));
        *state1.lock() = Some(2);
        assert_eq!(*state1.lock(), Some(2));

        let state2 = state1.clone();
        assert_eq!(*state2.lock(), Some(2));
        state1.set(Some(3));
        assert_eq!(*state1.lock(), Some(3)); // state1 is changed
        assert_eq!(*state2.lock(), Some(2)); // state2 is not changed
    }
}
