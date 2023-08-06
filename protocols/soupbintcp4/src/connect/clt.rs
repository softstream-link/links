use links_network_async::prelude::*;

use super::protocol::SBCltProtocol;

pub type SBClt<PAYLOAD, C, const MMS: usize> = Clt<SBCltProtocol<PAYLOAD>, C, MMS>;

#[cfg(test)]
mod test {

    use log::info;

    use crate::prelude::*;
    use links_testing::unittest::setup;

    const MMS: usize = 128;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();

        let callback = SBCltLoggerCallback::<SamplePayload>::new_ref(log::Level::Info);
        // let protocol = SBProtocol::<SamplePayload>::new();
        let clt = SBClt::<SamplePayload, _, MMS>::connect(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            callback,
            None,
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
