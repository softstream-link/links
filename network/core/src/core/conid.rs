use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub enum ConId {
    Clt {
        name: String,
        local: Option<SocketAddr>,
        peer: SocketAddr,
    },
    Svc {
        name: String,
        local: SocketAddr,
        peer: Option<SocketAddr>,
    },
}
impl ConId {
    pub fn clt(name: Option<&str>, local: Option<&str>, peer: &str) -> Self {
        ConId::Clt {
            name: name.unwrap_or("unknown").to_owned(),
            local: local.map(|addr| {
                addr.parse()
                    .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", addr))
            }),
            peer: peer
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", peer)),
        }
    }
    pub fn set_local(&mut self, local: SocketAddr) {
        match self {
            ConId::Clt { local: l, .. } => *l = Some(local),
            ConId::Svc { local: l, .. } => *l = local,
        }
    }
    pub fn set_peer(&mut self, peer: SocketAddr) {
        match self {
            ConId::Clt { peer: p, .. } => *p = peer,
            ConId::Svc { peer: p, .. } => *p = Some(peer),
        }
    }

    pub fn svc(name: Option<&str>, local: &str, peer: Option<&str>) -> Self {
        ConId::Svc {
            name: name.unwrap_or("unknown").to_owned(),
            local: local
                .parse()
                .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", local)),
            peer: peer.map(|addr| {
                addr.parse()
                    .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", addr))
            }),
        }
    }
    pub fn name(&self) -> &str {
        match self {
            ConId::Clt { name, .. } => name,
            ConId::Svc { name, .. } => name,
        }
    }
    pub fn get_peer(&self) -> Option<SocketAddr> {
        match self {
            ConId::Clt { peer, .. } => Some(*peer),
            ConId::Svc { peer, .. } => *peer,
        }
    }
    pub fn get_local(&self) -> Option<SocketAddr> {
        match self {
            ConId::Clt { local, .. } => *local,
            ConId::Svc { local, .. } => Some(*local),
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
            ConId::Clt { name, local, peer } => {
                write!(
                    f,
                    "Clt({name}@{}->{peer})",
                    match local {
                        Some(local) => format!("{}", local),
                        None => "pending".to_owned(),
                    }
                )
            }
            ConId::Svc { name, local, peer } => {
                write!(
                    f,
                    "Svc({name}@{local}<-{})",
                    match peer {
                        Some(peer) => format!("{}", peer),
                        None => "pending".to_owned(),
                    }
                )
            }
        }
    }
}

#[cfg(test)]
mod test {

    use log::info;

    use crate::prelude::*;
    use links_testing::unittest::setup;

    #[test]
    fn test_con_id() {
        setup::log::configure();
        let con_id = ConId::clt(Some("unittest"), None, "0.0.0.0:1");
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        assert_eq!(con_id.to_string(), "Clt(unittest@pending->0.0.0.0:1)");

        let con_id = ConId::svc(Some("unittest"), "0.0.0.0:1", None);
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        assert_eq!(con_id.to_string(), "Svc(unittest@0.0.0.0:1<-pending)");
    }
}
