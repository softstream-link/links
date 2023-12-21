use spin::MutexGuard;
use std::sync::Arc;

/// This facility helps user capture and maintain protocol state across a single connection.
///
/// # Use cases
/// * Use this to maintain login state
/// * Use this to maintain heart beat state
/// * etc..
///
/// # Key Features
/// * It is useful because all [crate::prelude::Protocol] methods are called with an immutable `&self` reference and to maintain state
/// user must use interior mutability.
/// * Interior mutability is implemented using [Arc<spin::Mutex>] for performance reasons. Note that different protocol methods can
/// potentially be invoked from different threads. Ex: [crate::prelude::ProtocolCore::on_sent] and [crate::prelude::ProtocolCore::on_recv]
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
    fn test_protocol_session_state() {
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
