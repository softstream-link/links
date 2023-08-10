use links_soupbintcp_async::prelude::*;

use crate::prelude::{Ouch5CltPld, Ouch5SvcPld};

pub type Ouch5CltProtocol = SBCltAdminProtocol<Ouch5CltPld>;
pub type Ouch5SvcProtocol = SBSvcAdminProtocol<Ouch5SvcPld>;