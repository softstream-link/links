use links_soupbintcp_async::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type Ouch5EventStore = EventStore<Ouch5Msg>;

pub type Ouch5CltEvenStoreCallback = EventStoreCallback<Ouch5Msg, Ouch5CltAdminProtocol>;
pub type Ouch5SvcEvenStoreCallback = EventStoreCallback<Ouch5Msg, Ouch5SvcAdminProtocol>;

// loggers
pub type Ouch5CltLoggerCallback = SBCltLoggerCallback<Ouch5CltAdminProtocol>;
pub type Ouch5SvcLoggerCallback = SBSvcLoggerCallback<Ouch5SvcAdminProtocol>;

// // chain
pub type Ouch5CltChainCallback = SBCltChainCallback<Ouch5CltAdminProtocol>;
pub type Ouch5SvcChainCallback = SBSvcChainCallback<Ouch5SvcAdminProtocol>;

// dev null
pub type Ouch5CltDevNullCallback = SBCltDevNullCallback<Ouch5CltAdminProtocol>;
pub type Ouch5SvcDevNullCallback = SBSvcDevNullCallback<Ouch5SvcAdminProtocol>;
