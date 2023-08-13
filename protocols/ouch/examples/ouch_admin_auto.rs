use std::time::Duration;

use lazy_static::lazy_static;
use links_ouch_async::prelude::*;
use links_testing::unittest::setup;
use log::{info, Level};

#[tokio::test]
async fn test() {
    test_clt_svc().await;
}

#[tokio::main]
async fn main() {
    test_clt_svc().await;
}

lazy_static! {
    static ref ADDR: &'static str = &setup::net::default_addr();
    static ref TMOUT: Duration = setup::net::default_connect_timeout();
    static ref RETRY: Duration = setup::net::default_connect_retry_after();
}

async fn test_clt_svc() {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_clbk = OuchSvcLoggerCallback::new_ref(Level::Info, Level::Debug);
    let svc_admin_prcl = OuchSvcAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        1.,
    );
    let svc = OuchSvc::bind(*ADDR, svc_clbk, svc_admin_prcl, Some("venue"))
        .await
        .unwrap();
    let svc_is_connected = svc.is_connected(None).await;
    info!("{} Status connected: {}", svc, svc_is_connected);
    assert!(!svc_is_connected);

    let clt_clbk = OuchCltLoggerCallback::new_ref(Level::Info, Level::Debug);

    info!("\n**********************************  AUTH OK  **********************************\n");
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
        Some("clt-pass"),
    )
    .await;

    assert!(clt.is_ok());
    let clt = clt.unwrap();
    let clt_connected = clt.is_connected(Duration::from_millis(100).into()).await;
    info!("{} Status connected: {}", clt, clt_connected);
    assert!(clt_connected);

    let svc_connected = svc.is_connected(Duration::from_millis(100).into()).await;
    info!("{} Status connected: {}", svc, svc_connected);
    assert!(svc_connected);

    drop(clt);
}
