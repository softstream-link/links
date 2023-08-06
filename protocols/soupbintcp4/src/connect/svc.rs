use links_network_async::prelude::*;

use crate::prelude::*;

pub type SBSvc<PAYLOAD, CALLBACK, const MMS: usize> = Svc<SBSvcProtocol<PAYLOAD>, CALLBACK, MMS>;

#[cfg(test)]
mod test {

    use std::{sync::Arc, time::Duration};

    use crate::prelude::*;
    use lazy_static::lazy_static;
    use links_network_async::prelude::*;
    use links_testing::unittest::setup;
    use log::{info, Level};

    type Msg = SBCltMsg<SamplePayload>;

    const MMS: usize = 128;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        let callback = SBSvcLoggerCallback::<SamplePayload>::new_ref(Level::Info);
        let svc = SBSvc::<_, _, MMS>::bind(
            &setup::net::default_addr(),
            callback,
            None,
            Some("soupbin/unittest"),
        )
        .await
        .unwrap();
        info!("{:?} connected", svc);
        assert!(!svc.is_accepted().await)
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure();
        // let find_timeout = setup::net::optional_find_timeout();
        let event_store = SBEventStore::new_ref();
        let svc_callback = SBSvcChainCallback::new_ref(vec![
            SBSvcLoggerCallback::new_ref(Level::Info),
            SBSvcEvenStoreCallback::new_ref(Arc::clone(&event_store)),
        ]);
        let clt_callback = SBCltChainCallback::new_ref(vec![
            SBCltLoggerCallback::new_ref(Level::Info),
            SBCltEvenStoreCallback::new_ref(Arc::clone(&event_store)),
        ]);

        let svc = SBSvc::<SamplePayload, _, MMS>::bind(
            &ADDR,
            svc_callback,
            None,
            Some("soupbin/venue"),
        )
        .await
        .unwrap();

        info!("{} started", svc);

        let clt = SBClt::<SamplePayload, _, MMS>::connect(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            clt_callback,
            None,
            Some("soupbin/broker"),
        )
        .await
        .unwrap();
        info!("{} started", clt);

        while !svc.is_accepted().await {}

        // let msg_clt = Msg::dbg(b"hello from client");
        // let msg_svc = Msg::dbg(b"hello from server");
        // clt.send(&msg_clt).await.unwrap();
        // svc.send(&msg_svc).await.unwrap();

        // let found = event_log
        //     .find(
        //         |entry| {
        //             let hit = match &entry.payload {
        //                 Dir::Recv(msg) => msg == &msg_svc,
        //                 _ => false,
        //             };

        //             let src = match &entry.con_id {
        //                 ConId::Clt { name, .. } | ConId::Svc { name, .. } => {
        //                     name == "soupbin/broker"
        //                 }
        //             };
        //             hit && src
        //         },
        //         find_timeout.into(),
        //     )
        //     .await;
        // info!("event_log: {}", *event_log);
        // info!("found: {:?}", found);
        // assert!(found.is_some());
        // assert_eq!(&msg_svc, found.unwrap().try_into_recv().unwrap());
    }
}
