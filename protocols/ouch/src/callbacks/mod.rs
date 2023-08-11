use links_soupbintcp_async::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type Ouch5EventStore = EventStore<SBMsg<OuchCltPld, OuchSvcPld>>;

pub type OuchCltEvenStoreCallback = SBCltEvenStoreCallback<OuchCltPld, OuchSvcPld, OuchCltAdminProtocol>;
pub type OuchSvcEvenStoreCallback = SBSvcEvenStoreCallback<OuchSvcPld, OuchCltPld, OuchSvcAdminProtocol>;

// loggers
pub type OuchCltLoggerCallback = SBCltLoggerCallback<OuchCltAdminProtocol>;
pub type OuchSvcLoggerCallback = SBSvcLoggerCallback<OuchSvcAdminProtocol>;

// // chain
pub type OuchCltChainCallback = SBCltChainCallback<OuchCltAdminProtocol>;
pub type OuchSvcChainCallback = SBSvcChainCallback<OuchSvcAdminProtocol>;

// dev null
pub type OuchCltDevNullCallback = SBCltDevNullCallback<OuchCltAdminProtocol>;
pub type OuchSvcDevNullCallback = SBSvcDevNullCallback<OuchSvcAdminProtocol>;
