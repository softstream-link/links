use links_soupbintcp_async::prelude::*;

use crate::prelude::*;

// event store
pub type Ouch5EventStore = SBEventStore<Ouch5Msg>;

pub type Ouch5CltEvenStoreCallback = SBCltEvenStoreCallback<Ouch5CltMsg, Ouch5CltProtocol>;
pub type Ouch5SvcEvenStoreCallback = SBCltEvenStoreCallback<Ouch5SvcMsg, Ouch5SvcProtocol>;

// loggers
pub type Ouch5CltLoggerCallback = SBCltLoggerCallback<Ouch5CltProtocol>;
pub type Ouch5SvcLoggerCallback = SBSvcLoggerCallback<Ouch5SvcProtocol>;

// // chain
pub type Ouch5CltChainCallback = SBCltChainCallback<Ouch5CltProtocol>;
pub type Ouch5SvcChainCallback = SBSvcChainCallback<Ouch5SvcProtocol>;

// dev null
pub type Ouch5CltDevNullCallback = SBCltDevNullCallback<Ouch5CltProtocol>;
pub type Ouch5SvcDevNullCallback = SBSvcDevNullCallback<Ouch5SvcProtocol>;
