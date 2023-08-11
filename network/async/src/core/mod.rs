pub mod conid;
pub mod counters;

use std::{
    error::Error,
    fmt::Debug,
    future::Future,
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

use crate::prelude::{CallbackSendRecv, Clt, CltSender};

use self::conid::ConId;

/// Provides a function that is meant to determine when enough bytes are available to make up a single complete message/frame.
pub trait Framer {
    /// The implementation of this function should use protocol specific logic to determine when enough bytes are available
    /// and return the Some(Bytes) or None per below
    /// ```
    /// // if required_frame_len = frame_len {
    /// //     let frame = bytes.split_to(required_frame_len);
    /// //     Some(frame.freeze())
    /// // } else {
    /// //     None
    /// // }
    /// ```
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

/// Provides a two types that a peer in the connection can send or recv if the message types are the same 
/// in both direction, just set to that same type in the implementation
#[rustfmt::skip]
pub trait Messenger: Debug + Send + Sync + 'static 
{
    type SendT: ByteDeserializeSlice<Self::SendT> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
    type RecvT: ByteDeserializeSlice<Self::RecvT> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
}

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
        async { false }
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
        clt: CltSender<P, C, MMS>,
    ) -> impl Future<Output=Result<(), Box<dyn Error+Send+Sync>>>+Send+'_ {
        async { Ok(()) }
    }

    fn on_recv<'s>(
        &'s self,
        con_id: &'s ConId,
        msg: &'s Self::RecvT,
    ) -> impl Future<Output=()>+Send+'_ {
        async {  }
    }
    fn on_send<'s>(&'s self, con_id: &'s ConId, msg: &'s mut Self::SendT) {} // TODO CRITICAL async not finished
}
