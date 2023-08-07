use links_soupbintcp_async::prelude::*;

use crate::prelude::{Ouch5CltMsg, Ouch5SvcMsg};

pub type Ouch5InbProtocolHandler = SBCltAdminProtocol<Ouch5CltMsg>;
pub type Ouch5OubProtocolHandler = SBCltAdminProtocol<Ouch5SvcMsg>;