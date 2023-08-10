use links_soupbintcp_async::prelude::*;

use crate::prelude::{Ouch5CltMsg, Ouch5SvcMsg};

pub type Ouch5CltProtocol = SBCltAdminProtocol<Ouch5CltMsg>;
pub type Ouch5SvcProtocol = SBSvcAdminProtocol<Ouch5SvcMsg>;