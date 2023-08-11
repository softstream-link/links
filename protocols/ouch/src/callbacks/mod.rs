use links_soupbintcp_async::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type Ouch5EventStore = EventStore<SBMsg<OuchCltPld, OuchSvcPld>>;

pub type OuchCltEvenStoreCallback<Messenger> = SBCltEvenStoreCallback<OuchCltPld, OuchSvcPld, Messenger>;
pub type OuchSvcEvenStoreCallback<Messenger> = SBSvcEvenStoreCallback<OuchCltPld, OuchSvcPld, Messenger>;

// loggers
pub type OuchCltLoggerCallback<Messenger> = SBCltLoggerCallback<Messenger>;
pub type OuchSvcLoggerCallback<Messenger> = SBSvcLoggerCallback<Messenger>;

// // chain
pub type OuchCltChainCallback<Messenger> = SBCltChainCallback<Messenger>;
pub type OuchSvcChainCallback<Messenger> = SBSvcChainCallback<Messenger>;

// dev null
pub type OuchCltDevNullCallback<Messenger> = SBCltDevNullCallback<Messenger>;
pub type OuchSvcDevNullCallback<Messenger> = SBSvcDevNullCallback<Messenger>;
