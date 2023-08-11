use links_soupbintcp_async::prelude::*;

use crate::prelude::{Ouch5CltPld, Ouch5SvcPld};

pub type Ouch5CltAdminProtocol = SBCltAdminProtocol<Ouch5CltPld>;
pub type Ouch5SvcAdminProtocol = SBSvcAdminProtocol<Ouch5SvcPld>;