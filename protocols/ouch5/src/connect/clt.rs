use soupbintcp4::prelude::*;

use super::messaging::Ouch5InbProtocolHandler;
use crate::model::ouch5::MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB;

pub type Ouch5Clt<CALLBACK> =
    SBClt<Ouch5InbProtocolHandler, CALLBACK, MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB>;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use lazy_static::lazy_static;
    use soupbintcp4::prelude::*;
    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
        static ref FIND_TIMEOUT: Duration = setup::net::default_find_timeout();
    }
    use crate::{prelude::*, unittest::setup};
    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        // let clt = Ouch5Clt::<_>::new();
        let clb = SBLoggerCallbackRef::<Ouch5Inb>::default();
        let clt = SBClt::<SoupBinProtocolHandler<Ouch5Inb>, _, MAX_FRAME_SIZE_OUCH5_INB>::new(
            *ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            clb,
            Some("ouch5/broker")
        )
        .await
        .unwrap();
    }
}
