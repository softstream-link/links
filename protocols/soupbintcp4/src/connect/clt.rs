use links_network_async::prelude::*;

use super::protocol::SBProtocol;

pub type SBClt<PAYLOAD, C, const MMS: usize> =
    Clt<SBProtocol<PAYLOAD>, C, MMS>;

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

    const MMS: usize = 128;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();

        let callback = SBLoggerCallback::<SamplePayload>::new_ref(log::Level::Info);
        let protocol = SBProtocol::<SamplePayload>::new();
        let clt = SBClt::<SamplePayload, _, MMS>::connect(
            setup::net::default_addr(),
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
