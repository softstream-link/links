use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type SBEventStore<PAYLOAD> = EventStore<SBMsg<PAYLOAD>>;

pub type SBCltEvenStoreCallback<PAYLOAD, PROTOCOL> = EventStoreCallback<SBMsg<PAYLOAD>, PROTOCOL>;

pub type SBSvcEvenStoreCallback<PAYLOAD, PROTOCOL> = EventStoreCallback<SBMsg<PAYLOAD>, PROTOCOL>;

// loggers
pub type SBCltLoggerCallback<PROTOCOL> = LoggerCallback<PROTOCOL>;
pub type SBSvcLoggerCallback<PROTOCOL> = LoggerCallback<PROTOCOL>;

// chain
pub type SBCltChainCallback<PROTOCOL> = ChainCallback<PROTOCOL>;
pub type SBSvcChainCallback<PROTOCOL> = ChainCallback<PROTOCOL>;

// dev null
pub type SBCltDevNullCallback<PROTOCOL> = DevNullCallback<PROTOCOL>;
pub type SBSvcDevNullCallback<PROTOCOL> = DevNullCallback<PROTOCOL>;
