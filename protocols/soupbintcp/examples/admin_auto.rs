use std::time::Duration;

use lazy_static::lazy_static;
use links_soupbintcp_async::prelude::*;
use links_testing::unittest::setup;
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
    static ref ADDR: &'static str = setup::net::default_addr();
    static ref TMOUT: Duration = setup::net::default_connect_timeout();
    static ref RETRY: Duration = setup::net::default_connect_retry_after();
}
const MMS: usize = MAX_FRAME_SIZE_SOUPBIN_EXC_PAYLOAD_DEBUG;
async fn test_clt_svc() {
    setup::log::configure_at(log::LevelFilter::Info);
    let svc_callback = SBSvcLoggerCallback::new_ref(Level::Info, Level::Debug);
    let svc_admin_protocol = SBSvcAdminProtocol::<NoPayload>::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
    );
    let svc = SBSvc::<_, _, MMS>::bind(*ADDR, svc_callback, svc_admin_protocol, Some("venue"))
        .await
        .unwrap();
    info!("{} started", svc);

    let clt_cb = SBCltLoggerCallback::new_ref(Level::Info, Level::Debug);
    info!("\n**********************************  AUTH ERROR  **********************************\n");
    let clt_pr = SBCltAdminProtocol::<NoPayload>::new_ref(
        b"abcdef".into(),
        b"----------".into(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let clt = SBClt::<_, _, MMS>::connect(
        *ADDR,
        *TMOUT,
        *RETRY,
        clt_cb.clone(),
        clt_pr,
        Some("clt-fail"),
    )
    .await;
    assert!(clt.is_err());
    error!("{} failed", clt.unwrap_err());

    info!("\n**********************************  AUTH OK  **********************************\n");
    let clt_pr = SBCltAdminProtocol::<NoPayload>::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        Default::default(),
        250.into(),
    );
    let clt = SBClt::<_, _, MMS>::connect(
        *ADDR,
        *TMOUT,
        *RETRY,
        clt_cb.clone(),
        clt_pr,
        Some("clt-pass"),
    )
    .await;

    assert!(clt.is_ok());
    let clt = clt.unwrap();
    info!("{} started", clt);
    tokio::time::sleep(Duration::from_millis(1)).await;
    drop(clt);
}
