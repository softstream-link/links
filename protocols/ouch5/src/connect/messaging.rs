use links_soupbintcp4::prelude::*;

use crate::prelude::{Ouch5Inb, Ouch5Oub};

pub type Ouch5InbProtocolHandler = SBCltAdminProtocol<Ouch5Inb>;
pub type Ouch5OubProtocolHandler = SBCltAdminProtocol<Ouch5Oub>;