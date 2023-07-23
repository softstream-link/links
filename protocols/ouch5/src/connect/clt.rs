use soupbintcp4::prelude::*;

use crate::{model::ouch5::MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB, prelude::Ouch5Inb};

pub type Ouch5Clt<CALLBACK> = SBClt<Ouch5Inb, CALLBACK, MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB>;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use lazy_static::lazy_static;
    use log::info;
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
        let callback = SBLoggerCallbackRef::<Ouch5Inb>::default();
        let clt = Ouch5Clt::new(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            callback,
            Some("ouch5/broker"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
