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
    fn on_connect<C: SendNonBlocking<<Self as Messenger>::SendT> + RecvNonBlocking<<Self as Messenger>::RecvT> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
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

/// This facility helps user capture and maintain protocol state across a single connection.
///
/// # Use cases
/// * Use this to maintain login state
/// * Use this to maintain heart beat state
/// * etc..
///
/// # Key Features
/// * It is useful because all [Protocol] methods are called with an immutable `&self` reference and to maintain state
/// user must use interior mutability.
/// * Interior mutability is implemented using [Arc<spin::Mutex>] for performance reasons. Note that different protocol methods can
/// potentially be invoked from different threads. Ex: [ProtocolCore::on_sent] and [ProtocolCore::on_recv]
/// * Because each new connections to the [crate::prelude::Svc] requires a copy of the `pristine` state in the original `user initialized`
/// form that is not affected by previous connections to [crate::prelude::Svc] the [Clone] implementation of this facility will clone `T`
/// instead of its [Arc] container so that connection cannot poison each other's state.
///
/// # Connection
/// * A connection is a single IP:PORT<->IP:PORT pair. In other word if the [crate::prelude::Clt] disconnects
/// and reconnects to the same [crate::prelude::Svc] port then it is considered a new connection, because each
/// new [crate::prelude::Clt] gets a new/unique/random port assigned by the OS to establish a connection to a [crate::prelude::Svc],
/// even though the [crate::prelude::Svc] port is the same.
///
/// ## Example of 2 connections
/// * Connection 1 - Clt(localhost:11111)->Svc(localhost:8080)
/// * Connection 2 - Clt(localhost:22222)->Svc(localhost:8080)
#[derive(Debug)]
pub struct ProtocolConnectionState<T: Clone>(Arc<spin::Mutex<T>>);
impl<T: Clone> ProtocolConnectionState<T> {
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
impl<T: Clone + Default> Default for ProtocolConnectionState<T> {
    fn default() -> Self {
        Self(Arc::new(spin::Mutex::new(T::default())))
    }
}
impl<T: Clone> Clone for ProtocolConnectionState<T> {
    /// Will provide a new [Arc] instance with a clone of `T` in its user initialized state.
    fn clone(&self) -> Self {
        Self(Arc::new(spin::Mutex::new(self.lock().clone())))
    }
}

/// This facility helps user capture and maintain protocol state across a single connection.
/// Unlike [ProtocolConnectionState] this facility designed to works across a session which spans across multiple connected/disconnected connections
/// 
/// # Use cases
/// * Use this to maintain a log of messages sent and received
/// * Use this to recover [crate::prelude::Clt] state after reconnect to [crate::prelude::Svc] since 
/// [crate::prelude::Svc] will have access to all of the activity across all connections.
/// 
/// # Key features
/// * Each new connection to [crate::prelude::Svc] will have access to any state captured by prior or still active connections
/// on the same [crate::prelude::Svc] port. This is achieved by using [Clone] implementation of [Arc]'s container for `T`.
#[derive(Debug)]
pub struct ProtocolSessionState<T>(Arc<spin::Mutex<T>>);
impl<T> ProtocolSessionState<T> {
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
impl<T: Default> Default for ProtocolSessionState<T> {
    fn default() -> Self {
        Self(Arc::new(spin::Mutex::new(T::default())))
    }
}
impl<T> Clone for ProtocolSessionState<T> {
    /// Will provide a shared reference to the same state of `T`
    fn clone(&self) -> Self {
        ProtocolSessionState(Arc::clone(&self.0))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_protocol_connection_state() {
        let state1 = ProtocolConnectionState::<Option<usize>>::default();
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

    #[test]
    fn test_protocol_session_state(){
        let state1 = ProtocolSessionState::<Option<usize>>::default();
        assert_eq!(*state1.lock(), None);
        state1.set(Some(1));
        assert_eq!(*state1.lock(), Some(1));
        *state1.lock() = Some(2);
        assert_eq!(*state1.lock(), Some(2));

        let state2 = state1.clone();
        assert_eq!(*state2.lock(), Some(2));
        state1.set(Some(3));
        assert_eq!(*state1.lock(), Some(3)); // state1 is changed
        assert_eq!(*state2.lock(), Some(3)); // state2 is also changed
    }
}
