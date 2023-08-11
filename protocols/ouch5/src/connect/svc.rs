use links_soupbintcp_async::prelude::*;

use crate::{model::ouch5::MAX_FRAME_SIZE_OUCH5_SVC_MSG, prelude::Ouch5SvcPld};

pub type Ouch5Svc<C> = SBSvc<SBSvcAdminProtocol<Ouch5SvcPld>, C, MAX_FRAME_SIZE_OUCH5_SVC_MSG>;

#[cfg(test)]
mod test {

    use std::time::Duration;

    use lazy_static::lazy_static;
    use links_testing::unittest::setup;
    use log::{info, Level};

    lazy_static! {
        static ref ADDR: &'static str = setup::net::default_addr();
    }
    use crate::prelude::*;

    #[tokio::test]
    async fn test_svc_not_connected() {
        setup::log::configure();
        let protocol = Ouch5SvcProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
        );
        let callback = Ouch5SvcLoggerCallback::new_ref(Level::Info, Level::Info);
        let svc = Ouch5Svc::bind(&ADDR, callback, protocol, Some("ouch5/venue"))
            .await
            .unwrap();
        info!("{}", svc);
        assert!(!svc.is_connected(None).await);
        assert!(!svc.is_accepted().await);
    }

    #[tokio::test]
    async fn test_svc_clt_connected() {
        setup::log::configure_at(log::LevelFilter::Info);
        let svc_prcl = Ouch5SvcProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            1.,
        );
        let clt_prcl = Ouch5CltProtocol::new_ref(
            b"abcdef".into(),
            b"++++++++++".into(),
            Default::default(),
            Default::default(),
            Duration::from_millis(1000),
            1.,
        );
        let svc_clbk = Ouch5SvcLoggerCallback::new_ref(Level::Info, Level::Debug);
        let clt_clbk = Ouch5CltLoggerCallback::new_ref(Level::Info, Level::Debug);
        //     let event_store = Ouch5EventStore::new_ref();
        //     let svc_clbk = Ouch5SvcEvenStoreCallback::new_ref(Arc::clone(&event_store));
        //     // let clt_clbk = Ouch5CltEvenStoreCallback::new_ref(Arc::clone(&event_store));

        let svc = Ouch5Svc::bind(*ADDR, svc_clbk, svc_prcl, Some("ouch5/venue"))
            .await
            .unwrap();

        info!("STARTED {}", svc);
        let clt = Ouch5Clt::connect(
            *ADDR,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            clt_clbk,
            clt_prcl,
            Some("ouch5/broker"),
        ).await.unwrap();
        info!("STARTED {}", clt);
        
        let clt_is_connected = clt.is_connected(Some(Duration::from_millis(500))).await;
        let svc_is_connected = svc.is_connected(Some(Duration::from_secs(500))).await;
        assert!(clt_is_connected);
        assert!(svc_is_connected);
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
