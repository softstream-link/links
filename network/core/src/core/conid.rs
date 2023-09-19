use std::{fmt::Display, net::SocketAddr};

#[derive(Debug, Clone, PartialEq)]
pub enum ConId {
    CltCon {
        name: String,
        local: Option<SocketAddr>,
        peer: SocketAddr,
    },
    SvcCon {
        name: String,
        local: SocketAddr,
        peer: Option<SocketAddr>,
    },
}
impl ConId {
    pub fn clt(name: Option<&str>, local: Option<&str>, peer: &str) -> Self {
        ConId::CltCon {
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
            ConId::CltCon { local: l, .. } => *l = Some(local),
            ConId::SvcCon { local: l, .. } => *l = local,
        }
    }
    pub fn set_peer(&mut self, peer: SocketAddr) {
        match self {
            ConId::CltCon { peer: p, .. } => *p = peer,
            ConId::SvcCon { peer: p, .. } => *p = Some(peer),
        }
    }

    pub fn svc(name: Option<&str>, local: &str, peer: Option<&str>) -> Self {
        ConId::SvcCon {
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
            ConId::CltCon { name, .. } => name,
            ConId::SvcCon { name, .. } => name,
        }
    }
    pub fn get_peer(&self) -> Option<SocketAddr> {
        match self {
            ConId::CltCon { peer, .. } => Some(*peer),
            ConId::SvcCon { peer, .. } => *peer,
        }
    }
    pub fn get_local(&self) -> Option<SocketAddr> {
        match self {
            ConId::CltCon { local, .. } => *local,
            ConId::SvcCon { local, .. } => Some(*local),
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
            ConId::CltCon { name, local, peer } => {
                write!(
                    f,
                    "CltCon({name}@{}->{peer})",
                    match local {
                        Some(local) => format!("{}", local),
                        None => "pending".to_owned(),
                    }
                )
            }
            ConId::SvcCon { name, local, peer } => {
                write!(
                    f,
                    "SvcCon({name}@{local}<-{})",
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
        assert_eq!(con_id.to_string(), "CltCon(unittest@pending->0.0.0.0:1)");

        let con_id = ConId::svc(Some("unittest"), "0.0.0.0:1", None);
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        assert_eq!(con_id.to_string(), "SvcCon(unittest@0.0.0.0:1<-pending)");
    }
}
