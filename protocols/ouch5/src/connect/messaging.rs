use links_soupbintcp4::prelude::*;

use crate::prelude::{Ouch5Inb, Ouch5Oub};

pub type Ouch5InbProtocolHandler = SoupBinProtocolHandler<Ouch5Inb>;
pub type Ouch5OubProtocolHandler = SoupBinProtocolHandler<Ouch5Oub>;