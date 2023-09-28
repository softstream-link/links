use links_soupbintcp_async::prelude::*;

use crate::prelude::MAX_FRAME_SIZE_OUCH_CLT_MSG;

pub type OuchClt<Protocol, Callback> = SBClt<Protocol, Callback, MAX_FRAME_SIZE_OUCH_CLT_MSG>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use links_network_core::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::rand_avail_addr_port();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let prcl = OuchCltAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            Default::default(),
            1.,
        );
        let clbk = OuchCltLoggerCallback::new_ref(Level::Info, Level::Info);
        let res = OuchClt::connect_async(
            &ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clbk,
            Some(prcl),
            Some("ouch5/broker"),
        )
        .await;
        info!("{:?}", res);
        assert!(res.is_err());
    }
}
