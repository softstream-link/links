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
    static ref ADDR: &'static str = &setup::net::default_addr();
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
    let svc = OuchSvc::bind(*ADDR, svc_clbk, svc_prcl, Some("ouch/venue"))
        .await
        .unwrap();
    let svc_is_connected = svc.is_connected(None).await;
    info!("{} Status connected: {}", svc, svc_is_connected);
    assert!(!svc_is_connected);

    let clt_prcl = OuchCltAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        Default::default(),
        Duration::from_millis(250),
        1.,
    );
    let clt = OuchClt::connect(
        *ADDR,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        clt_clbk.clone(),
        clt_prcl,
        Some("ouch/broker"),
    )
    .await
    .unwrap();

    let clt_connected = clt.is_connected(Duration::from_millis(100).into()).await;
    info!("{} Status connected: {}", clt, clt_connected);
    assert!(clt_connected);

    let svc_connected = svc.is_connected(Duration::from_millis(100).into()).await;
    info!("{} Status connected: {}", svc, svc_connected);
    assert!(svc_connected);
}

// // SEND A NEW ORDER
        // let enter_order = EnterOrder::default();
        // clt.send(&mut enter_order.clone().into()).await.unwrap();

        // // FIND THIS ORDER RECVED IN BY THE SVC VIA EVENT_STORE
        // let search = event_store.find_recv(
        //     |msg| matches!(msg, OuchMsg::Clt(OuchCltMsg::U(UPayload{payload: OuchCltPld::Enter(ord), ..})) if ord == &enter_order),
        //     setup::net::optional_find_timeout()).await;

        // warn!("{:?}", search.unwrap());

        // //
        // let accepted = OrderAccepted::from(&enter_order);
        // svc.send(&mut accepted.into()).await.unwrap();

        
