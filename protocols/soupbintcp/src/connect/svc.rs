use links_network_async::prelude::*;

pub type SBSvc<PROTOCOL, CALLBACK, const MMS: usize> = Svc<PROTOCOL, CALLBACK, MMS>;

#[cfg(test)]
mod test {

    use std::{sync::Arc, time::Duration};

    use crate::prelude::*;
    use lazy_static::lazy_static;
    use links_network_async::prelude::*;
    use links_testing::unittest::setup;
    use log::{info, Level};

    const MMS: usize = 128;

    lazy_static! {
        static ref ADDR: &'static str = &setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
    }

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        
        let svc = SBSvc::<_, _, MMS>::bind_no_protocol(
            *ADDR,
            SBSvcLoggerCallback::<SBSvcAdminProtocol<NoPayload>>::new_ref(Level::Info, Level::Info),
            Some("soupbin/unittest"),
        )
        .await
        .unwrap();
        info!("{:?} connected", svc);
        assert!(!svc.is_accepted().await)
    }

    #[tokio::test]
    async fn test_svc_clt_connected() {
        setup::log::configure();

        let event_store = SBEventStore::new_ref();
        let svc_callback = SBSvcChainCallback::<SBSvcAdminProtocol<NoPayload>>::new_ref(vec![
            SBSvcLoggerCallback::new_ref(Level::Info, Level::Info),
            SBSvcEvenStoreCallback::new_ref(Arc::clone(&event_store)),
        ]);
        let clt_callback = SBCltChainCallback::<SBCltAdminProtocol<NoPayload>>::new_ref(vec![
            SBCltLoggerCallback::new_ref(Level::Info, Level::Info),
            SBCltEvenStoreCallback::new_ref(Arc::clone(&event_store)),
        ]);

        let svc = SBSvc::<_, _, MMS>::bind_no_protocol(*ADDR, svc_callback, Some("soupbin/venue"))
            .await
            .unwrap();

        info!("{} started", svc);

        let clt = SBClt::<_, _, MMS>::connect_no_protocol(
            *ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            clt_callback,
            Some("soupbin/broker"),
        )
        .await
        .unwrap();
        info!("{} started", clt);

        while !svc.is_accepted().await {}

        let mut msg_clt_inp = SBCltMsg::dbg(b"hello from client");
        let mut msg_svc_inp = SBSvcMsg::dbg(b"hello from server");
        clt.send(&mut msg_clt_inp).await.unwrap();
        svc.send(&mut msg_svc_inp).await.unwrap();

        let msg_clt_out = event_store
            .find(
                |entry| match &entry.event {
                    Dir::Recv(SBMsg::Svc(msg)) => msg == &msg_svc_inp,
                    _ => false,
                },
                setup::net::optional_find_timeout(),
            )
            .await
            .unwrap()
            .unwrap_recv_event();
        let msg_svc_out = event_store
            .find(
                |entry| match &entry.event {
                    Dir::Recv(SBMsg::Clt(msg)) => msg == &msg_clt_inp,
                    _ => false,
                },
                setup::net::optional_find_timeout(),
            )
            .await
            .unwrap()
            .unwrap_recv_event();
        info!("event_store: {}", *event_store);
        info!("msg_svc_out: {:?}", msg_svc_out);
        info!("msg_clt_out: {:?}", msg_clt_out);
    }
}
