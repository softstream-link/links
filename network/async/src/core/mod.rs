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
                .expect(format!("unable to parse addr: {:?}", local).as_str()),
            peer: peer
                .parse()
                .expect(format!("unable to parse addr: {:?}", peer).as_str()),
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
                .expect(format!("unable to parse addr: {:?}", local).as_str()),
            peer: peer
                .unwrap_or("0.0.0.0:0")
                .parse()
                .expect(format!("unable to parse addr: {:?}", peer).as_str()),
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

pub trait Framer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}

#[rustfmt::skip]
pub trait Messenger: Debug + Clone + Send + Sync + 'static 
// where
//     TARGET: From<Self::SendMsg> + From<Self::RecvMsg> + Debug + Clone + PartialEq + Send + Sync +,
{
    type SendMsg: ByteDeserializeSlice<Self::SendMsg> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;
    type RecvMsg: ByteDeserializeSlice<Self::RecvMsg> + ByteSerializeStack + Debug + Clone + PartialEq + Send + Sync + 'static;

}

pub trait Protocol: Messenger + Framer + Send + Sync + 'static {}

#[cfg(test)]
mod test {

    use log::info;

    use crate::unittest::setup;

    use crate::prelude::*;

    #[test]
    fn test_cond_id() {
        setup::log::configure();
        let con_id = ConId::default();
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
    }
}
