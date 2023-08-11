use links_soupbintcp_async::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type Ouch5EventStore = EventStore<SBMsg<OuchCltPld, OuchSvcPld>>;

pub type OuchCltEvenStoreCallback<Protocol> = SBCltEvenStoreCallback<OuchCltPld, OuchSvcPld, Protocol>;
pub type OuchSvcEvenStoreCallback<Protocol> = SBSvcEvenStoreCallback<OuchCltPld, OuchSvcPld, Protocol>;

// loggers
pub type OuchCltLoggerCallback<Protocol> = SBCltLoggerCallback<Protocol>;
pub type OuchSvcLoggerCallback<Protocol> = SBSvcLoggerCallback<Protocol>;

// // chain
pub type OuchCltChainCallback<Protocol> = SBCltChainCallback<Protocol>;
pub type OuchSvcChainCallback<Protocol> = SBSvcChainCallback<Protocol>;

// dev null
pub type OuchCltDevNullCallback<Protocol> = SBCltDevNullCallback<Protocol>;
pub type OuchSvcDevNullCallback<Protocol> = SBSvcDevNullCallback<Protocol>;
