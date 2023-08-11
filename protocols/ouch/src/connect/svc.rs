use links_soupbintcp_async::prelude::*;

use crate::model::ouch::MAX_FRAME_SIZE_OUCH_SVC_MSG;

pub type OuchSvc<PROTOCOL, CALLBACK> = SBSvc<PROTOCOL, CALLBACK, MAX_FRAME_SIZE_OUCH_SVC_MSG>;

#[cfg(test)]
mod test {

    use std::{sync::Arc, time::Duration};

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::default_addr();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let protocol = OuchSvcAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
        );
        let callback = OuchSvcLoggerCallback::new_ref(Level::Info, Level::Info);
        let svc = OuchSvc::bind(&ADDR, callback, protocol, Some("ouch5/venue"))
            .await
            .unwrap();
        info!("{}", svc);
        assert!(!svc.is_connected(None).await);
        assert!(!svc.is_accepted().await);
    }

    #[tokio::test]
    async fn test_svc_clt_connected() {
        setup::log::configure_level(log::LevelFilter::Info);
        let svc_prcl = OuchSvcAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            1.,
        );
        let clt_prcl = OuchCltAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            Duration::from_millis(1000),
            1.,
        );
        // let svc_clbk = Ouch5SvcLoggerCallback::new_ref(Level::Info, Level::Debug);
        // let clt_clbk = Ouch5CltLoggerCallback::new_ref(Level::Info, Level::Debug);

        let event_store = Ouch5EventStore::new_ref();
        let svc_clbk = OuchSvcEvenStoreCallback::new_ref(Arc::clone(&event_store));
        let clt_clbk = OuchCltEvenStoreCallback::new_ref(Arc::clone(&event_store));

        let svc = OuchSvc::bind(*ADDR, svc_clbk, svc_prcl, Some("ouch5/venue"))
            .await
            .unwrap();

        info!("STARTED {}", svc);
        let clt = OuchClt::connect(
            *ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clt_clbk,
            clt_prcl,
            Some("ouch5/broker"),
        )
        .await
        .unwrap();
        info!("STARTED {}", clt);

        let clt_is_connected = clt.is_connected(Some(Duration::from_millis(500))).await;
        let svc_is_connected = svc.is_connected(Some(Duration::from_secs(500))).await;
        assert!(clt_is_connected);
        assert!(svc_is_connected);
        info!("event_store: {}", event_store);
        //   // tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
