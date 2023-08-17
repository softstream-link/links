use byteserde::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_ouch_async::prelude::*;
use links_soupbintcp_async::prelude::Debug;

fn ouch_order_ser(c: &mut Criterion) {
    let order = EnterOrder::new(
        1.into(),
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
    // let msg = UPayload::new(order); // 9ns

    // let msg: OuchCltMsg = order.into();
    ///////
    // let msg = order; // 9ns
    // let msg = OuchCltPld::Enter(order); // 10ns
    // let msg = UPayload::new(OuchCltPld::Enter(order)); // 10ns
    let msg = OuchCltMsg::U(UPayload::new(OuchCltPld::Enter(order))); // 28ns // TODO WHY??
    

    c.bench_function("ouch_order_ser", |b| {
        b.iter(|| {
            black_box({
                let _: ([u8; MAX_FRAME_SIZE_OUCH_CLT_MSG], usize) = to_bytes_stack(&msg).unwrap();
                // let _: ([u8; 51], usize) = to_bytes_stack(&msg).unwrap();
            })
        })
    });
}

fn ouch_order_des(c: &mut Criterion) {
    let order = EnterOrder::new(
        1.into(),
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
    // let msg = UPayload::new(order); // 9ns

    // let msg: OuchCltMsg = order.into();
    ///////
    // let msg = order; // 9ns
    // let msg = OuchCltPld::Enter(order); // 10ns
    // let msg = UPayload::new(OuchCltPld::Enter(order)); // 10ns
    let msg = OuchCltMsg::U(UPayload::new(OuchCltPld::Enter(order))); // 28ns // TODO WHY??
    let (arr, len): ([u8; MAX_FRAME_SIZE_OUCH_CLT_MSG], usize) = to_bytes_stack(&msg).unwrap();

    c.bench_function("ouch_order_des", |b| {
        b.iter(|| {
            black_box({
                let msg: OuchCltMsg = from_slice(&arr[..len]).unwrap();
            })
        })
    });
}

criterion_group!(benches, ouch_order_ser, ouch_order_des);
criterion_main!(benches);
