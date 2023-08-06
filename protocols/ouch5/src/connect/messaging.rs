use links_soupbintcp4::prelude::*;

use crate::prelude::{Ouch5Inb, Ouch5Oub};

pub type Ouch5InbProtocolHandler = SBProtocol<Ouch5Inb>;
pub type Ouch5OubProtocolHandler = SBProtocol<Ouch5Oub>;