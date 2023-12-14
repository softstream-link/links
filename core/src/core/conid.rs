use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub enum ConId {
    Initiator { name: String, local: Option<SocketAddr>, peer: SocketAddr },
    Acceptor { name: String, local: SocketAddr, peer: Option<SocketAddr> },
}
impl ConId {
    pub fn clt(name: Option<&str>, local: Option<&str>, peer: &str) -> Self {
        ConId::Initiator {
            name: name.unwrap_or("unknown").to_owned(),
            local: local.map(|addr| addr.parse().unwrap_or_else(|_| panic!("unable to parse addr: {:?}", addr))),
            peer: peer.parse().unwrap_or_else(|_| panic!("unable to parse addr: {:?}", peer)),
        }
    }
    pub fn set_local(&mut self, local: SocketAddr) {
        match self {
            ConId::Initiator { local: l, .. } => *l = Some(local),
            ConId::Acceptor { local: l, .. } => *l = local,
        }
    }
    pub fn set_peer(&mut self, peer: SocketAddr) {
        match self {
            ConId::Initiator { peer: p, .. } => *p = peer,
            ConId::Acceptor { peer: p, .. } => *p = Some(peer),
        }
    }

    pub fn svc(name: Option<&str>, local: &str, peer: Option<&str>) -> Self {
        ConId::Acceptor {
            name: name.unwrap_or("unknown").to_owned(),
            local: local.parse().unwrap_or_else(|_| panic!("unable to parse addr: {:?}", local)),
            peer: peer.map(|addr| addr.parse().unwrap_or_else(|_| panic!("unable to parse addr: {:?}", addr))),
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ConId::Initiator { name, .. } => name,
            ConId::Acceptor { name, .. } => name,
        }
    }
    pub fn get_peer(&self) -> Option<SocketAddr> {
        match self {
            ConId::Initiator { peer, .. } => Some(*peer),
            ConId::Acceptor { peer, .. } => *peer,
        }
    }
    pub fn get_local(&self) -> Option<SocketAddr> {
        match self {
            ConId::Initiator { local, .. } => *local,
            ConId::Acceptor { local, .. } => Some(*local),
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
            ConId::Initiator { name, local, peer } => {
                write!(
                    f,
                    "Initiator({name}@{}->{peer})",
                    match local {
                        Some(local) => format!("{}", local),
                        None => "pending".to_owned(),
                    }
                )
            }
            ConId::Acceptor { name, local, peer } => {
                write!(
                    f,
                    "Acceptor({name}@{local}<-{})",
                    match peer {
                        Some(peer) => format!("{}", peer),
                        None => "pending".to_owned(),
                    }
                )
            }
        }
    }
}

pub trait ConnectionId {
    fn con_id(&self) -> &ConId;
}

pub trait ConnectionStatus {
    /// logical check of connection status
    fn is_connected(&self) -> bool;
}
pub trait PoolConnectionStatus {
    fn is_next_connected(&mut self) -> bool;
    fn all_connected(&mut self) -> bool;
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::prelude::*;
    use crate::unittest::setup;

    #[test]
    fn test_con_id() {
        setup::log::configure();
        let con_id = ConId::clt(Some("unittest"), None, "0.0.0.0:1");
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        assert_eq!(con_id.to_string(), "Initiator(unittest@pending->0.0.0.0:1)");

        let con_id = ConId::svc(Some("unittest"), "0.0.0.0:1", None);
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        assert_eq!(con_id.to_string(), "Acceptor(unittest@0.0.0.0:1<-pending)");
    }
}
