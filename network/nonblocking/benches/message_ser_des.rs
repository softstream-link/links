use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::prelude::Messenger;
use links_network_nonblocking::{
    unittest::setup::messenger::TestCltMsgProtocol,
    unittest::setup::{framer::TEST_MSG_FRAME_SIZE, messenger::TestSvcMsgProtocol},
};
use links_testing::unittest::setup::{
    self,
    model::{TestCltMsg, TestCltMsgDebug},
};

fn serialize_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let id = format!("serialize TestCltMsg");
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                // create msg during benchmarking otherwise --> AnalyzingCriterion.rs ERROR: At least one measurement of benchmark serialize TestCltMsg took zero time per iteration
                let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
                let _x = TestCltMsgProtocol::serialize::<TEST_MSG_FRAME_SIZE>(&msg).unwrap();
            })
        })
    });
}

fn deserialize_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    let (buf, len) = TestCltMsgProtocol::serialize::<TEST_MSG_FRAME_SIZE>(&msg).unwrap();
    let buf = &buf[..len];
    let id = format!("deserialize TestCltMsg");
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let _x = TestSvcMsgProtocol::deserialize(buf).unwrap();
            })
        })
    });
}

criterion_group!(benches, serialize_msg, deserialize_msg,);
// criterion_group!(benches, recv_random_frame);
criterion_main!(benches);
