use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_core::{
    prelude::Messenger,
    unittest::setup::{self, messenger::*},
};

fn serialize_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let id = format!("serialize TestCltMsg");
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                // create msg during benchmarking otherwise --> AnalyzingCriterion.rs ERROR: At least one measurement of benchmark serialize TestCltMsg took zero time per iteration
                let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
                let _x = CltTestMessenger::serialize::<TEST_MSG_FRAME_SIZE>(&msg).unwrap();
            })
        })
    });
}

fn deserialize_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    let (buf, len) = CltTestMessenger::serialize::<TEST_MSG_FRAME_SIZE>(&msg).unwrap();
    let buf = &buf[..len];
    let id = format!("deserialize TestCltMsg");
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let _x = SvcTestMessenger::deserialize(buf).unwrap();
            })
        })
    });
}

criterion_group!(benches, serialize_msg, deserialize_msg,);
// criterion_group!(benches, recv_random_frame);
criterion_main!(benches);