
use links_network_async::prelude::*;

use crate::prelude::*;


// loggers
pub type SBCltLoggerCallback<PAYLOAD> = LoggerCallback<SBCltProtocol<PAYLOAD>>;
pub type SBSvcLoggerCallback<PAYLOAD> = LoggerCallback<SBSvcProtocol<PAYLOAD>>;

// dev null
// TODO

// event store
pub type SBEventStore<PAYLOAD> = EventStoreRef<SBMsg<PAYLOAD>>;
pub type SBCltEvenStoreCallback<PAYLOAD> = EventStoreCallback<SBMsg<PAYLOAD>, SBCltProtocol<PAYLOAD>>;
pub type SBSvcEvenStoreCallback<PAYLOAD> = EventStoreCallback<SBMsg<PAYLOAD>, SBSvcProtocol<PAYLOAD>>;

// chain
pub type SBCltChainCallbackRef<PAYLOAD> = ChainCallback<SBCltProtocol<PAYLOAD>>;
pub type SBSvcChainCallbackRef<PAYLOAD> = ChainCallback<SBSvcProtocol<PAYLOAD>>;
