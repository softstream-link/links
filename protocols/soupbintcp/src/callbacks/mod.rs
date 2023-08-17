use links_network_async::prelude::*;

use crate::prelude::*;

// event store
pub type SBEventStore<CltPayload, SvcPayload> = EventStoreAsync<SBMsg<CltPayload, SvcPayload>>;

pub type SBCltEvenStoreCallback<CltPayload, SvcPayload, Messenger> =
    EventStoreCallback<SBMsg<CltPayload, SvcPayload>, Messenger>;

pub type SBSvcEvenStoreCallback<CltPayload, SvcPayload, Messenger> =
    EventStoreCallback<SBMsg<CltPayload, SvcPayload>, Messenger>;

// loggers
pub type SBCltLoggerCallback<Messenger> = LoggerCallback<Messenger>;
pub type SBSvcLoggerCallback<Messenger> = LoggerCallback<Messenger>;

// chain
pub type SBCltChainCallback<Messenger> = ChainCallback<Messenger>;
pub type SBSvcChainCallback<Messenger> = ChainCallback<Messenger>;

// dev null
pub type SBCltDevNullCallback<Messenger> = DevNullCallback<Messenger>;
pub type SBSvcDevNullCallback<Messenger> = DevNullCallback<Messenger>;
