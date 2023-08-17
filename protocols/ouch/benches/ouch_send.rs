use std::{sync::Arc, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_ouch_async::prelude::*;
use links_testing::unittest::setup;
use log::info;
use tokio::runtime::Builder;

fn ouch_order_send(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());
    let addr = setup::net::rand_avail_addr_port();
    // CONFIGURE
    let svc_prcl = OuchSvcAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        1.,
    );
    let clt_hbeat_inverval = Duration::from_millis(1000);
    let clt_prcl = OuchCltAdminProtocol::new_ref(
        b"abcdef".into(),
        b"++++++++++".into(),
        Default::default(),
        Default::default(),
        clt_hbeat_inverval,
        1.,
    );

    // let svc_clbk = OuchSvcDevNullCallback::new_ref();
    // let clt_clbk = OuchCltDevNullCallback::new_ref();
    let svc_clbk = OuchSvcCounterCallback::new_ref();
    let clt_clbk = OuchCltCounterCallback::new_ref();

    // START
    let svc = OuchSvc::bind_sync(
        addr,
        Arc::clone(&svc_clbk),
        Some(svc_prcl),
        Some("ouch5/venue"),
        runtime.clone(),
    )
    .unwrap();

    let clt = OuchClt::connect_sync(
        addr,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        Arc::clone(&clt_clbk),
        Some(clt_prcl),
        Some("ouch5/clt"),
        runtime.clone(),
    )
    .unwrap();
    assert!(clt.is_connected(Some(clt_hbeat_inverval)));
    assert!(svc.is_connected(Some(clt_hbeat_inverval)));

    info!("STARTED {}", clt);
    info!("STARTED {}", svc);

    let mut user_ref_number_generator = UserRefNumberGenerator::default();
    c.bench_function("ouch_order_send", |b| {
        b.iter(|| {
            black_box({
                let mut msg = get_order(&mut user_ref_number_generator);
                clt.send(&mut msg).unwrap()
            })
        })
    });

    info!("clt_clbk: {}", clt_clbk);
    info!("svc_clbk: {}", svc_clbk);
    
}
fn get_order(user_ref: &mut UserRefNumberGenerator) -> OuchCltMsg {
    let order = EnterOrder::new(
        user_ref.next().unwrap(),
        100.into(),
        b"IBM".as_slice().into(),
        15.into(),
        TimeInForce::market_hours(),
        Display::visible(),
        Capacity::agency(),
        IntMktSweepEligibility::eligible(),
        CrossType::continuous_market(),
        b"MY ORDER ID #1".as_slice().into(),
        OptionalAppendage::default(),
    );
    let msg: OuchCltMsg = order.into();
    msg
}

criterion_group!(benches, ouch_order_send);
criterion_main!(benches);
