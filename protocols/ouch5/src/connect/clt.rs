use links_soupbintcp4::prelude::*;

use crate::{model::ouch5::MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB, prelude::Ouch5Inb};

pub type Ouch5Clt<C> = SBClt<Ouch5Inb, C, MAX_FRAME_SIZE_SOUPBIN_OUCH5_INB>;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::info;
    use links_soupbintcp4::prelude::*;
    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let callback = SBCltLoggerCallback::<Ouch5Inb>::default();
        let clt = Ouch5Clt::connect(
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
