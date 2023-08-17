use std::{sync::Arc, time::Duration};

use lazy_static::lazy_static;
use links_ouch_async::prelude::*;
use links_testing::unittest::setup;
use log::{info, Level};

#[tokio::test]
async fn test() {
    test_clt_svc_connect().await;
}

#[tokio::main]
async fn main() {
    test_clt_svc_connect().await;
}

lazy_static! {
    static ref ADDR: &'static str = &setup::net::rand_avail_addr_port();
}

async fn test_clt_svc_connect() {
    setup::log::configure_level(log::LevelFilter::Info);

    let event_store = OuchEventStore::new_ref();
    // log only recv & store
    let svc_clbk = OuchSvcChainCallback::new_ref(vec![
        OuchSvcLoggerCallback::new_ref(Level::Info, Level::Debug),
        OuchSvcEvenStoreCallback::new_ref(Arc::clone(&event_store)),
    ]);
    let clt_clbk = OuchCltChainCallback::new_ref(vec![
        OuchCltLoggerCallback::new_ref(Level::Info, Level::Debug),
        OuchCltEvenStoreCallback::new_ref(Arc::clone(&event_store)),
    ]);

    let svc_prcl = OuchSvcAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        1.,
    );
    // START SVC
    let svc = OuchSvc::bind(*ADDR, svc_clbk, svc_prcl, Some("ouch/venue"))
        .await
        .unwrap();
    let svc_is_connected = svc.is_connected(None).await;
    info!("{} Status connected: {}", svc, svc_is_connected);
    assert!(!svc_is_connected);

    let hbeat_interval = Duration::from_millis(250);
    let clt_prcl = OuchCltAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        Default::default(),
        hbeat_interval,
        1.,
    );
    // START CLT
    let clt = OuchClt::connect_async(
        *ADDR,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        clt_clbk.clone(),
        Some(clt_prcl),
        Some("ouch/broker"),
    )
    .await
    .unwrap();

    // VALIDATED BOTH CONNECTED
    let clt_connected = clt.is_connected(hbeat_interval.into()).await;
    info!("{} Status connected: {}", clt, clt_connected);
    assert!(clt_connected);

    let svc_connected = svc.is_connected(hbeat_interval.into()).await;
    info!("{} Status connected: {}", svc, svc_connected);
    assert!(svc_connected);

    // SEND A NEW ORDER
    let mut enter_order = EnterOrder::default().into();
    clt.send(&mut enter_order).await.unwrap();

    let ouch_msg  = event_store
        .find_recv(
            svc.con_id().name(),
            |sb_msg| 
                matches!(sb_msg, OuchMsg::Clt(OuchCltMsg::U(UPayload{body: OuchCltPld::Enter(ord), ..})) if ord.user_ref_number == UserRefNumber::new(1)),
            setup::net::optional_find_timeout(),
        )
        .await.unwrap();
    
    let enter_order: &EnterOrder = ouch_msg.unwrap_clt_u().try_into().unwrap();
    
    let mut order_accepted= OrderAccepted::from(enter_order).into();
    svc.send(&mut order_accepted).await.unwrap();


    let _  = event_store
        .find_recv(
            clt.con_id().name(),
            |sb_msg| 
                matches!(sb_msg, OuchMsg::Svc(OuchSvcMsg::U(UPayload{body: OuchSvcPld::Accepted(ord), ..})) if ord.user_ref_number == UserRefNumber::new(1)),
            setup::net::optional_find_timeout(),
        )
        .await.unwrap();
    
    info!("event_store: {}", event_store);
}

