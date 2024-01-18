pub use crate::core::{
    framer::{into_split_framer, FrameReader, FrameWriter},
    messenger::{into_split_messenger, MessageRecver, MessageSender},
    protocol::{
        persistance::{FileMessageLog, InMemoryMessageLog, ProtocolStorage},
        state::{ProtocolConnectionState, ProtocolSessionState},
        Protocol, ProtocolCore,
    },
    AcceptStatus, PollAble, PollAccept, PollEventStatus, PollRead, PoolAcceptStatus, PoolSvcAcceptorOfCltNonBlocking, ReSendNonBlocking, RecvNonBlocking, RecvStatus, SendNonBlocking, SendNonBlockingNonMut, SendStatus, SvcAcceptorOfCltNonBlocking,
};

pub use crate::connect::{
    clt::{Clt, CltRecver, CltRecverRef, CltSender, CltSenderRef},
    poll::{PollHandler, PollHandlerDynamic, PollHandlerStatic, SpawnedPollHandler, SpawnedPollHandlerDynamic, SpawnedPollHandlerStatic},
    pool::{CltRecversPool, CltSendersPool, CltsPool, TransmittingSvcAcceptor, TransmittingSvcAcceptorRef},
    svc::{Svc, SvcAcceptor, SvcRecver, SvcRecverRef, SvcSender, SvcSenderRef},
};

pub use links_core::prelude::*;

// #[cfg(feature = "unittest")]
// pub use crate::unittest::{self }; // doing this will shadow the unittest module in links_core
