use std::{time::Duration, };

use lazy_static::lazy_static;
use links_network_async::prelude::Clt;
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
    setup::log::configure_at(log::LevelFilter::Info);
    let svc_callback = SBSvcLoggerCallback::new_ref(Level::Error);
    let svc_admin_protocol = SBSvcAdminAutoProtocol::new_ref(
        b"abcdef".into(),
        b"1234567890".into(),
        Default::default(),
    );
    let svc = SBSvcAdminAuto::<NoPayload, _, 128>::bind(
        *ADDR,
        svc_callback,
        Some(svc_admin_protocol),
        Some("venue"),
    )
    .await.unwrap();
    info!("{} started", svc);

    let clt_callback = SBCltLoggerCallback::new_ref(Level::Warn);
    let clt_admin_protocol = SBCltAdminAutoProtocol::<NoPayload>::new_ref(
        b"abcdef".into(),
        b"1234567890".into(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    let clt = Clt::<_, _, 128>::connect(
        *ADDR,
        *CONNECT_TIMEOUT,
        *RETRY_AFTER,
        clt_callback,
        clt_admin_protocol,
        Some("broker"),
    ).await.unwrap();
    // let clt = SBCltAdminAuto::<NoPayload, _, 128>::connect_opt_protocol(
    //     *ADDR,
    //     *CONNECT_TIMEOUT,
    //     *RETRY_AFTER,
    //     clt_callback,
    //     Some(clt_admin_protocol),
    //     Some("broker"),
    // ).await.unwrap();
    info!("{} started", clt);
    tokio::time::sleep(Duration::from_millis(100)).await;
}
