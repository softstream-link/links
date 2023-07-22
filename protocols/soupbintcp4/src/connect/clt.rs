use links_network_async::prelude::*;

use super::messaging::SoupBinProtocolHandler;

pub type SBClt<PAYLOAD, CALLBACK, const MAX_MSG_SIZE: usize> = Clt<SoupBinProtocolHandler<PAYLOAD>, CALLBACK, MAX_MSG_SIZE>;