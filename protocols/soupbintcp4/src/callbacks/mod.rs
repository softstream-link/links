
use links_network_async::prelude::*;

use crate::connect::protocol::SoupBinProtocolHandler;

pub type SBLoggerCallbackRef<PAYLOAD> = LoggerCallbackRef<SoupBinProtocolHandler<PAYLOAD>>;
pub type SBEvenLogCallbackRef<PAYLOAD> = EventLogCallbackRef<SoupBinProtocolHandler<PAYLOAD>>;
pub type SBChainCallbackRef<PAYLOAD> = ChainCallbackRef<SoupBinProtocolHandler<PAYLOAD>>;
