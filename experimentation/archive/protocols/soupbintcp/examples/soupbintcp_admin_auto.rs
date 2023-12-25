use std::time::Duration;

use lazy_static::lazy_static;
use links_core::unittest::setup;
use links_soupbintcp_async::prelude::*;
use log::{error, info, Level};

#[tokio::test]
async fn test() {
    test_clt_svc().await;
}

#[tokio::main]
async fn main() {
    test_clt_svc().await;
}

lazy_static! {
    static ref ADDR: &'static str = &setup::net::rand_avail_addr_port();
    static ref TMOUT: Duration = setup::net::default_connect_timeout();
    static ref RETRY: Duration = setup::net::default_connect_retry_after();
}
const MMS: usize = MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
async fn test_clt_svc() {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_clbk = SBSvcLoggerCallback::new_ref(Level::Info, Level::Debug);
    let svc_prcl_admin = SBSvcAdminProtocol::<Nil, Nil>::new_ref(b"abcdef".into(), b"++++++++++".into(), Default::default(), 1.);
    let svc = SBSvc::<_, _, MMS>::bind(*ADDR, svc_clbk, Some(svc_prcl_admin), Some("venue")).await.unwrap();
    let svc_is_connected = svc.is_connected(None).await;
    info!("{} Status connected: {}", svc, svc_is_connected);
    assert!(!svc_is_connected);

    let clt_clbk = SBCltLoggerCallback::new_ref(Level::Info, Level::Debug);
    info!("\n**********************************  AUTH ERROR  **********************************\n");
    let clt_prcl_admin = SBCltAdminProtocol::<Nil, Nil>::new_ref(b"abcdef".into(), b"----------".into(), Default::default(), Default::default(), Default::default(), 1.);
    let clt = SBClt::<_, _, MMS>::connect(
        *ADDR,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        clt_clbk.clone(),
        Some(clt_prcl_admin),
        Some("clt-fail"),
    )
    .await;
    assert!(clt.is_err());
    error!("{} failed", clt.unwrap_err());

    let svc_is_connected = svc.is_connected(None).await;
    info!("{} Status connected: {}", svc, svc_is_connected);
    assert!(!svc_is_connected);

    info!("\n**********************************  AUTH OK  **********************************\n");
    let clt_prcl_admin = SBCltAdminProtocol::<Nil, Nil>::new_ref(b"abcdef".into(), b"++++++++++".into(), Default::default(), Default::default(), Duration::from_millis(250), 1.);
    let clt = SBClt::<_, _, MMS>::connect(
        *ADDR,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        clt_clbk.clone(),
        Some(clt_prcl_admin),
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
