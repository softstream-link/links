
use links_network_async::prelude::*;

use crate::connect::protocol::SoupBinProtocol;

pub type SBLoggerCallbackRef<PAYLOAD> = LoggerCallbackRef<SoupBinProtocol<PAYLOAD>>;
pub type SBEvenLogCallbackRef<PAYLOAD> = EventLogCallbackRef<SoupBinProtocol<PAYLOAD>>;
pub type SBChainCallbackRef<PAYLOAD> = ChainCallbackRef<SoupBinProtocol<PAYLOAD>>;
