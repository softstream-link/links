use links_soupbintcp_async::prelude::*;

use crate::model::ouch::MAX_FRAME_SIZE_OUCH_SVC_MSG;

pub type OuchSvc<Protocol, Callback> = SBSvc<Protocol, Callback, MAX_FRAME_SIZE_OUCH_SVC_MSG>;

#[cfg(test)]
mod test {

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::{info, Level};
    use std::{sync::Arc, time::Duration};

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::rand_avail_addr_port();
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

        // CONFIGURE
        let svc_prcl = OuchSvcAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            1.,
        );
        let clt_hbeat_inverval = Duration::from_millis(200);
        let clt_prcl = OuchCltAdminProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            clt_hbeat_inverval,
            1.,
        );
        // let svc_clbk = Ouch5SvcLoggerCallback::new_ref(Level::Info, Level::Debug);
        // let clt_clbk = Ouch5CltLoggerCallback::new_ref(Level::Info, Level::Debug);

        let event_store = OuchEventStore::new_ref();
        let svc_clbk = OuchSvcEvenStoreCallback::new_ref(Arc::clone(&event_store));
        let clt_clbk = OuchCltEvenStoreCallback::new_ref(Arc::clone(&event_store));

        // START
        let svc = OuchSvc::bind(*ADDR, svc_clbk, svc_prcl, Some("ouch5/venue"))
            .await
            .unwrap();

        info!("STARTED {}", svc);
        let clt = OuchClt::connect_async(
            *ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clt_clbk,
            Some(clt_prcl),
            Some("ouch5/broker"),
        )
        .await
        .unwrap();
        info!("STARTED {}", clt);

        // MAKE SURE CONNECTED
        // wait at least one heartbeat interval after opening connection
        let svc_is_connected = svc.is_connected(Some(clt_hbeat_inverval)).await; 
        let clt_is_connected = clt.is_connected(None).await;
        assert!(clt_is_connected);
        assert!(svc_is_connected);

        
        // REVIEW EVENTS
        info!("event_store: {}", event_store);

        // STOP clt and wait at least one heartbeat interval for svc to detect is_connected = false
        drop(clt);
        tokio::time::sleep(clt_hbeat_inverval).await; 
        // EXPECT this call shall generate log warn indicating hbeat is outside of tolerance factor
        let svc_is_connected = svc.is_connected(None).await;
        assert!(!svc_is_connected);
    }
}


