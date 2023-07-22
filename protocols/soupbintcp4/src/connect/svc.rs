use links_network_async::prelude::*;

use super::messaging::SoupBinProtocolHandler;

pub type SBSvc<PAYLOAD, CALLBACK, const MAX_MSG_SIZE: usize> =
    Svc<SoupBinProtocolHandler<PAYLOAD>, CALLBACK, MAX_MSG_SIZE>;

#[cfg(test)]
mod test {
    use super::*;
    use std::{sync::Arc, time::Duration};

    use lazy_static::lazy_static;
    use log::info;

    use crate::{prelude::*, unittest::setup};

    type Msg = SBMsg<SamplePayload>;

    const MAX_MSG_SIZE: usize = 128;

    lazy_static! {
        static ref ADDR: String = setup::net::default_addr();
        static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
        static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
        static ref FIND_TIMEOUT: Duration = setup::net::default_find_timeout();
    }

    #[tokio::test]
    async fn test_svc_clt_connection() {
        setup::log::configure();
        let find_timeout = setup::net::default_find_timeout();
        let event_log = SBEvenLogCallbackRef::default();
        let callback = SBChainCallbackRef::new(ChainCallback::new(vec![
            SBLoggerCallbackRef::default(),
            event_log.clone(),
        ]));

        let svc = SBSvc::<SamplePayload, _, MAX_MSG_SIZE>::new(
            &ADDR,
            Arc::clone(&callback),
            Some("soupbin/venue"),
        )
        .await
        .unwrap();

        info!("{} started", svc);

        let clt = SBClt::<SamplePayload, _, MAX_MSG_SIZE>::new(
            &ADDR,
            *CONNECT_TIMEOUT,
            *RETRY_AFTER,
            Arc::clone(&callback),
            Some("soupbin/broker"),
        )
        .await
        .unwrap();
        info!("{} started", clt);

        while !svc.is_accepted().await {}

        let msg_clt = Msg::dbg(b"hello from client");
        let msg_svc = Msg::dbg(b"hello from server");
        clt.send(&msg_clt).await.unwrap();
        svc.send(&msg_svc).await.unwrap();

        let found = event_log
            .find(
                |entry| {
                    entry.direction == Direction::Recv
                        && entry.msg == msg_svc
                        && match &entry.con_id {
                            ConId::Clt { name, .. } | ConId::Svc { name, .. } => {
                                name == "soupbin/broker"
                            }
                        }
                },
                find_timeout.into(),
            )
            .await;
        info!("event_log: {}", *event_log);
        info!("found: {:?}", found);
        assert!(found.is_some());
        assert_eq!(found.unwrap().msg, msg_svc);
    }
}
