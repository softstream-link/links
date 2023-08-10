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
            local: match local {
                None => None,
                Some(local) => Some(
                    local
                        .parse()
                        .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", local)),
                ),
            },
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
            peer: match peer {
                None => None,
                Some(peer) => Some(
                    peer.parse()
                        .unwrap_or_else(|_| panic!("unable to parse addr: {:?}", peer)),
                ),
            },
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
                        None => "none".to_owned(),
                    }
                )
            }
            ConId::Svc { name, local, peer } => {
                write!(
                    f,
                    "Svc({name}@{local}<-{})",
                    match peer {
                        Some(peer) => format!("{}", peer),
                        None => "none".to_owned(),
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
    fn test_cond_id() {
        setup::log::configure();
        let con_id = ConId::clt(Some("unittest"), None, "0.0.0.0:1");
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
        let con_id = ConId::svc(Some("unittest"), "0.0.0.0:1", None);
        info!("con_id: {:?}", con_id);
        info!("con_id: {}", con_id);
    }
}
