use links_async::prelude::*;

use crate::prelude::*;

// event store
pub type SBEventStoreAsync<CltPayload, SvcPayload> = EventStoreAsync<SBMsg<CltPayload, SvcPayload>>;
pub type SBEventStoreSync<CltPayload, SvcPayload> = EventStoreSync<SBMsg<CltPayload, SvcPayload>>;

pub type SBCltEvenStoreCallback<CltPayload, SvcPayload, Messenger> =
    EventStoreCallback<SBMsg<CltPayload, SvcPayload>, Messenger>;

pub type SBSvcEvenStoreCallback<CltPayload, SvcPayload, Messenger> =
    EventStoreCallback<SBMsg<CltPayload, SvcPayload>, Messenger>;

// loggers
pub type SBCltLoggerCallback<Messenger> = LoggerCallbackOld<Messenger>;
pub type SBSvcLoggerCallback<Messenger> = LoggerCallbackOld<Messenger>;

// chain
pub type SBCltChainCallback<Messenger> = ChainCallback<Messenger>;
pub type SBSvcChainCallback<Messenger> = ChainCallback<Messenger>;

// dev null
pub type SBCltDevNullCallback<Messenger> = DevNullCallbackOld<Messenger>;
pub type SBSvcDevNullCallback<Messenger> = DevNullCallbackOld<Messenger>;

// counters
pub type SBCltCounterCallback<Messenger> = CounterCallback<Messenger>;
pub type SBSvcCounterCallback<Messenger> = CounterCallback<Messenger>;

