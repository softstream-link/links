pub mod counters;

use std::{error::Error, future::Future, time::Duration};

use crate::prelude::*;

use links_network_core::prelude::{
    CallbackSendRecv, ConId, EventIntervalTracker, Framer, Messenger,
};

/// This trait brings the Framer and Messenger traits together as well as provides a series of functions
/// that can be used to enable automated reply logics or provide telemetry information on the connection
///
/// Why Clone?
///     Because specifically in the Svc connection case it is possible to accept several Clt's. Hence each Svc stream
/// needs to have own protocol instance.
///  
#[allow(unused_variables)]
pub trait Protocol: Clone+Messenger+Framer+Send+Sync+'static {
    /// Provides a protocol specific implementation of the connection status by analyzing packets going
    /// through the connection
    fn is_connected(&self, timeout: Option<Duration>) -> impl Future<Output=bool>+'_ {
        async {
            todo!(
            "
            Default implementation of this method is not provided.
            Typicall implementaiton involves you implementing {}::on_recv( ... ) and then track if the last message arrived 
            with in allowed tolerance Interval. {}::is_within_tolerance_factor() can be used to help track arrival frequency.
            ", std::any::type_name::<Self>()
            , std::any::type_name::<EventIntervalTracker>()
        )
        }
    }
    fn handshake<
        's,
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &'s self,
        clt: &'s Clt<P, C, MMS>,
    ) -> impl Future<Output=Result<(), Box<dyn Error+Send+Sync>>>+Send+'_ {
        async { Ok(()) }
    }

    fn keep_alive_loop<
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &self,
        clt: CltSenderAsync<P, C, MMS>,
    ) -> impl Future<Output=Result<(), Box<dyn Error+Send+Sync>>>+Send+'_ {
        async { Ok(()) }
    }

    #[inline(always)]
    fn on_recv<'s>(
        &'s self,
        con_id: &'s ConId,
        msg: &'s Self::RecvT,
    ) -> impl Future<Output=()>+Send+'_ {
        async {}
    }
    #[inline(always)]
    fn on_send<'s>(
        &'s self,
        con_id: &'s ConId,
        msg: &'s mut Self::SendT,
    ) -> impl Future<Output=()>+Send+'_ {
        async {}
    }
}
