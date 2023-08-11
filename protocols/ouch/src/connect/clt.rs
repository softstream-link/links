use links_soupbintcp_async::prelude::*;

use crate::prelude::{OuchCltAdminProtocol, MAX_FRAME_SIZE_OUCH_CLT_MSG};


pub type OuchClt<C> = SBClt<OuchCltAdminProtocol, C, MAX_FRAME_SIZE_OUCH_CLT_MSG>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::default_addr();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let protocol = OuchCltAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            Default::default(),
            1.,
        );
        let callback = OuchCltLoggerCallback::new_ref(Level::Info, Level::Info);
        let res = OuchClt::connect(
            &ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            callback,
            protocol,
            Some("ouch5/broker"),
        )
        .await;
        info!("{:?}", res);
        assert!(res.is_err());
    }
}
