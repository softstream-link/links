use links_network_async::prelude::*;

pub type SBClt<PROTOCOL, CALLBACK, const MMS: usize> = Clt<PROTOCOL, CALLBACK, MMS>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use log::{info, Level};

    use crate::prelude::*;
    use links_testing::unittest::setup;

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::default_addr();
    }

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();

        let clt = SBClt::<_, _, 128>::connect_no_protocol(
            *ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            SBCltLoggerCallback::<SBCltAdminProtocol<Nil, Nil>>::new_ref(Level::Info, Level::Info),
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
