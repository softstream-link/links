use links_network_async::prelude::*;
use links_soupbintcp_async::prelude::*;

use crate::prelude::*;

// event store
pub type OuchEventStoreAsync = EventStoreAsync<OuchMsg>;
pub type OuchEventStoreSync = EventStoreSync<OuchMsg>;

pub type OuchCltEvenStoreCallback<Messenger> = EventStoreCallback<OuchMsg, Messenger>;
pub type OuchSvcEvenStoreCallback<Messenger> = EventStoreCallback<OuchMsg, Messenger>;

// loggers
pub type OuchCltLoggerCallback<Messenger> = SBCltLoggerCallback<Messenger>;
pub type OuchSvcLoggerCallback<Messenger> = SBSvcLoggerCallback<Messenger>;

// // chain
pub type OuchCltChainCallback<Messenger> = SBCltChainCallback<Messenger>;
pub type OuchSvcChainCallback<Messenger> = SBSvcChainCallback<Messenger>;

// dev null
pub type OuchCltDevNullCallback<Messenger> = SBCltDevNullCallback<Messenger>;
pub type OuchSvcDevNullCallback<Messenger> = SBSvcDevNullCallback<Messenger>;

// counters
pub type OuchCltCounterCallback<Messenger> = SBCltCounterCallback<Messenger>;
pub type OuchSvcCounterCallback<Messenger> = SBSvcCounterCallback<Messenger>;
