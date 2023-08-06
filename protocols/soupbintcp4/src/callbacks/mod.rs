use links_network_async::{prelude::*, callbacks::devnull::DevNullCallback};

use crate::prelude::*;

// event store
pub type SBEventStore<PAYLOAD> = EventStore<SBMsg<PAYLOAD>>;

pub type SBCltEvenStoreCallback<PAYLOAD> =
    EventStoreCallback<SBMsg<PAYLOAD>, SBCltProtocol<PAYLOAD>>;

pub type SBSvcEvenStoreCallback<PAYLOAD> =
    EventStoreCallback<SBMsg<PAYLOAD>, SBSvcAdminAutoProtocol<PAYLOAD>>;

// loggers
pub type SBCltLoggerCallback<PAYLOAD> = LoggerCallback<SBCltProtocol<PAYLOAD>>;
pub type SBSvcLoggerCallback<PAYLOAD> = LoggerCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;

// chain
pub type SBCltChainCallback<PAYLOAD> = ChainCallback<SBCltProtocol<PAYLOAD>>;
pub type SBSvcChainCallback<PAYLOAD> = ChainCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;

// dev null
pub type SBCltDevNullCallback<PAYLOAD> = DevNullCallback<SBCltProtocol<PAYLOAD>>;
pub type SBSvcDevNullCallback<PAYLOAD> = DevNullCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;