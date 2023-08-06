use links_network_async::prelude::*;

use crate::prelude::*;

pub type SBClt<PAYLOAD, CALLBACK, const MMS: usize> = Clt<SBCltProtocol<PAYLOAD>, CALLBACK, MMS>;

#[cfg(test)]
mod test {

    use log::info;

    use crate::prelude::*;
    use links_testing::unittest::setup;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();

        let clt = SBClt::<SamplePayload, _, 128>::connect(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            SBCltLoggerCallback::new_ref(log::Level::Info),
            None,
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
