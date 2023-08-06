use std::time::Duration;

use lazy_static::lazy_static;
use links_soupbintcp4::prelude::*;
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
    static ref ADDR: &'static str = setup::net::default_addr();
    static ref CONNECT_TIMEOUT: Duration = setup::net::default_connect_timeout();
    static ref RETRY_AFTER: Duration = setup::net::default_connect_retry_after();
}

async fn test_clt_svc() {
    setup::log::configure();
    let svc_callback = SBSvcLoggerCallback::new_ref(Level::Info);
    let svc_admin_protocol = SBSvcAdminAutoProtocol::new_ref(
        b"abcdef".into(),
        b"1234567890".into(),
        b"session #1".into(),
    );
    let svc = SBSvc::<NoPayload, _, 128>::bind(
        *ADDR,
        svc_callback,
        Some(svc_admin_protocol),
        Some("soupbin/venue"),
    )
    .await.unwrap();
    info!("{} started", svc);
}
