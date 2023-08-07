use links_network_async::{prelude::*, callbacks::devnull::DevNullCallback};

use crate::prelude::*;

// event store
pub type SBEventStore<PAYLOAD> = EventStore<SBMsg<PAYLOAD>>;

pub type SBCltEvenStoreCallback<PAYLOAD> =
    EventStoreCallback<SBMsg<PAYLOAD>, SBCltAdminAutoProtocol<PAYLOAD>>;

pub type SBSvcEvenStoreCallback<PAYLOAD> =
    EventStoreCallback<SBMsg<PAYLOAD>, SBSvcAdminAutoProtocol<PAYLOAD>>;

// loggers
// pub type SBCltLoggerCallback<PAYLOAD> = LoggerCallback<SBCltAdminAutoProtocol<PAYLOAD>>;
pub type SBCltLoggerCallback<M> = LoggerCallback<M>;
pub type SBSvcLoggerCallback<PAYLOAD> = LoggerCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;

// chain
pub type SBCltChainCallback<PAYLOAD> = ChainCallback<SBCltAdminAutoProtocol<PAYLOAD>>;
pub type SBSvcChainCallback<PAYLOAD> = ChainCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;

// dev null
pub type SBCltDevNullCallback<PAYLOAD> = DevNullCallback<SBCltAdminAutoProtocol<PAYLOAD>>;
pub type SBSvcDevNullCallback<PAYLOAD> = DevNullCallback<SBSvcAdminAutoProtocol<PAYLOAD>>;