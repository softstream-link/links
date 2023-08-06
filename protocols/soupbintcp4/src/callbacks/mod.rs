
use links_network_async::prelude::*;

use crate::prelude::*;
// {connect::protocol::SoupBinProtocol, prelude::SBMsg};

pub type SBLoggerCallback<PAYLOAD> = LoggerCallback<SBProtocol<PAYLOAD>>;
pub type SBEvenLogCallback<PAYLOAD> = EventStoreCallback<SBCltMsg<PAYLOAD>, SBProtocol<PAYLOAD>>;
pub type SBChainCallbackRef<PAYLOAD> = ChainCallback<SBProtocol<PAYLOAD>>;
