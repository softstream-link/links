use links_network_async::prelude::*;

pub type SBClt<PROTOCOL, CALLBACK, const MMS: usize> = Clt<PROTOCOL, CALLBACK, MMS>;

#[cfg(test)]
mod test {

    use log::{info, Level};

    use crate::prelude::*;
    use links_testing::unittest::setup;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();

        let clt = SBClt::<_, _, 128>::connect_no_protocol(
            &setup::net::default_addr(),
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            SBCltLoggerCallback::<SBCltAdminProtocol<SamplePayload>>::new_ref(Level::Info, Level::Info),
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
