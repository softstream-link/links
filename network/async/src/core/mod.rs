use std::{
    fmt::{Debug, Display},
    net::SocketAddr,
};

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ConId {
    Clt {
        name: String,
        local: SocketAddr,
        peer: SocketAddr,
    },
    Svc {
        name: String,
        local: SocketAddr,
        peer: SocketAddr,
    },
}
impl ConId {
    pub fn clt(name: Option<&str>, local: Option<&str>, peer: &str) -> Self {
        ConId::Clt {
            name: name.unwrap_or("unknown").to_owned(),
            local: local
                .unwrap_or("0.0.0.0:0")
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", local)),
            peer: peer
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", peer)),
        }
    }
    pub fn set_local(&mut self, local: SocketAddr) {
        match self {
            ConId::Clt { local: l, .. } => *l = local,
            ConId::Svc { local: l, .. } => *l = local,
        }
    }
    pub fn set_peer(&mut self, peer: SocketAddr) {
        match self {
            ConId::Clt { peer: p, .. } => *p = peer,
            ConId::Svc { peer: p, .. } => *p = peer,
        }
    }

    pub fn svc(name: Option<&str>, local: &str, peer: Option<&str>) -> Self {
        ConId::Svc {
            name: name.unwrap_or("unknown").to_owned(),
            local: local
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", local)),
            peer: peer
                .unwrap_or("0.0.0.0:0")
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", peer)),
        }
    }
}
impl Default for ConId {
    fn default() -> Self {
        ConId::clt(None, None, "0.0.0.0:0")
    }
}
impl Display for ConId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConId::Clt { name, local, peer } => write!(f, "Clt({name}@{local}->{peer})"),
            ConId::Svc { name, local, peer } => write!(f, "Svc({name}@{local}<-{peer})"),
        }
    }
}
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
pub trait Messenger: Debug + Clone + Send + Sync + 'static 
{
    type SendMsg: ByteDeserializeSlice<Self::SendMsg> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
    type RecvMsg: ByteDeserializeSlice<Self::RecvMsg> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
}

/// This trait brings the Framer and Messenger traits together as well as provides a series of functions
/// that can be used to enable automated reply logics or provide telemetry information on the connection
pub trait Protocol: Messenger + Framer + Send + Sync + 'static {
    /// Provides a protocol specific implementation of the connection status by analyzing packets going
    /// through the connection
    fn is_connected(&self) -> bool {
        false
    }
    fn on_recv(&mut self, _con_id: &ConId, _msg: &Self::RecvMsg) {}
    fn on_send(&mut self, _con_id: &ConId, _msg: &mut Self::SendMsg) {}
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::prelude::*;
    use links_testing::unittest::setup;

    #[test]
    fn test_cond_id() {
        setup::log::configure();
        let con_id = ConId::default();
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
    }
}
