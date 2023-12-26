use links_async::prelude::*;

pub type SBClt<Protocol, Callback, const MMS: usize> = Clt<Protocol, Callback, MMS>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use log::{info, Level};

    use crate::prelude::*;
    use links_core::unittest::setup;

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::rand_avail_addr_port();
    }

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();

        let clt = SBClt::<_, _, 128>::connect(
            *ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            SBCltLoggerCallback::<SBCltAdminProtocol<Nil, Nil>>::new_ref(Level::Info, Level::Info),
            None,
            Some("soupbin/unittest"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
