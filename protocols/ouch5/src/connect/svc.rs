use links_soupbintcp_async::prelude::*;

use crate::{model::ouch5::MAX_FRAME_SIZE_OUCH5_SVC_MSG, prelude::Ouch5SvcMsg};

pub type Ouch5Svc<C> = SBSvc<SBSvcAdminProtocol<Ouch5SvcMsg>, C, MAX_FRAME_SIZE_OUCH5_SVC_MSG>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = setup::net::default_addr();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        let protocol = Ouch5SvcProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
        );
        let callback = Ouch5SvcLoggerCallback::new_ref(Level::Info, Level::Info);
        let svc = Ouch5Svc::bind(
            &ADDR,
            callback,
            protocol,
            Some("ouch5/venue"),
        )
        .await
        .unwrap();
        info!("{}", svc);
        // assert!(res.is_err());
    }
}
