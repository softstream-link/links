use links_soupbintcp_async::prelude::*;

use crate::{model::ouch5::MAX_FRAME_SIZE_OUCH5_CLT_MSG, prelude::Ouch5CltMsg};

pub type Ouch5Clt<C> = SBClt<SBCltAdminProtocol<Ouch5CltMsg>, C, MAX_FRAME_SIZE_OUCH5_CLT_MSG>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use links_soupbintcp_async::prelude::*;
    use links_testing::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = setup::net::default_addr();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let protocol = SBCltAdminProtocol::<Ouch5CltMsg>::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            Default::default(),
        );
        let callback = SBCltLoggerCallback::new_ref(Level::Info, Level::Info);
        let clt = Ouch5Clt::connect(
            &ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            callback,
            protocol,
            Some("ouch5/broker"),
        )
        .await;
        info!("{:?} not connected", clt);
        assert!(clt.is_err());
    }
}
