use links_network_async::prelude::*;

use super::protocol::SoupBinProtocolHandler;

pub type SBClt<PAYLOAD, CALLBACK, const MAX_MSG_SIZE: usize> =
    Clt<SoupBinProtocolHandler<PAYLOAD>, CALLBACK, MAX_MSG_SIZE>;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use lazy_static::lazy_static;

    use log::info;

    use crate::prelude::*;
    use links_testing::unittest::setup;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }

    const MAX_MSG_SIZE: usize = 128;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();

        let callback = SBLoggerCallbackRef::<SamplePayload>::default();

        let clt = SBClt::<SamplePayload, _, MAX_MSG_SIZE>::connect(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            callback,
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
