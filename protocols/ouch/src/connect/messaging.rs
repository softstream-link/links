use links_soupbintcp_async::prelude::*;

use crate::prelude::{OuchCltPld, OuchSvcPld};

pub type OuchCltAdminProtocol = SBCltAdminProtocol<OuchCltPld, OuchSvcPld>;
pub type OuchSvcAdminProtocol = SBSvcAdminProtocol<OuchSvcPld, OuchCltPld>;