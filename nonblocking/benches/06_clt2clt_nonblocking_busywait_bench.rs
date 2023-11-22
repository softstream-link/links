use std::{num::NonZeroUsize, sync::Arc, thread::Builder, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_core::{
    fmt_num,
    prelude::{CallbackRecvSend, DevNullCallback, Messenger},
    unittest::setup::{
        self,
        framer::TEST_MSG_FRAME_SIZE,
        model::{CltTestMsg, CltTestMsgDebug, SvcTestMsg, SvcTestMsgDebug},
    },
};
use links_nonblocking::{
    prelude::*,
    unittest::setup::protocol::{CltTestProtocolAuth, SvcTestProtocolAuth},
};
use log::info;

fn setup<MSvc: Messenger, MClt: Messenger>() -> (&'static str, Arc<impl CallbackRecvSend<MSvc>>, Arc<impl CallbackRecvSend<MClt>>, NonZeroUsize, Option<&'static str>, Duration, Duration) {
    let addr = setup::net::rand_avail_addr_port();
    let svc_callback = DevNullCallback::<MSvc>::new_ref();
    let clt_callback = DevNullCallback::<MClt>::new_ref();
    let name = Some("bench");
    let max_connections = NonZeroUsize::new(1).unwrap();
    let timeout = Duration::from_micros(1_000);
    let retry_after = timeout / 10;
    (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after)
}

fn send_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<SvcTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::bind(addr, svc_callback, max_connections, None, name.clone()).unwrap();

            // info!("svc: {}", svc);

            let mut clt_acceptor = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted();
            info!("clt_acceptor: {}", clt_acceptor);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            loop {
                match clt_acceptor.recv() {
                    Ok(RecvStatus::Completed(Some(_recv_msg))) => {
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(RecvStatus::Completed(None)) => {
                        info!("Connection Closed by clt_initiator clt_acceptor: {}", clt_acceptor);
                        break;
                    }
                    Ok(RecvStatus::WouldBlock) => continue,
                    Err(err) => {
                        panic!("Connection Closed by clt_initiator, clt_acceptor: {}, err: {}", clt_acceptor, err);
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let mut clt_initiator = Clt::<CltTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, None, name.clone()).unwrap();
    info!("clt_initiator: {}", clt_initiator);

    let id = format!("clt2clt_nonblocking_busywait_send_msg SvcTestMsg");
    let mut clt_initiator_msg_send_count = 0_usize;
    let mut clt_initiator_send_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator.send_busywait(&mut clt_initiator_send_msg).unwrap();
                clt_initiator_msg_send_count += 1;
            })
        })
    });

    drop(clt_initiator); // this will allow svc.join to complete
    let clt_acceptor_msg_recv_count = clt_acceptor_jh.join().unwrap();
    info!("clt_acceptor_msg_recv_count: {:?}, clt_initiator_msg_send_count: {:?}", fmt_num!(clt_acceptor_msg_recv_count), fmt_num!(clt_initiator_msg_send_count));
    assert_eq!(clt_initiator_msg_send_count, clt_acceptor_msg_recv_count);
}

fn recv_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    // CONFIGURE svc
    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<SvcTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::bind(addr, svc_callback, max_connections, None, name.clone()).unwrap();

            let mut clt_acceptor = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted();
            info!("clt_acceptor: {}", clt_acceptor);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            let mut clt_acceptor_msg_send = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));
            loop {
                match clt_acceptor.send(&mut clt_acceptor_msg_send) {
                    Ok(SendStatus::Completed) => {
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(SendStatus::WouldBlock) => continue,
                    Err(err) => {
                        info!("Connection Closed by clt_initiator, clt_acceptor: {}, err: {}", clt_acceptor, err);
                        break;
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let mut clt_initiator = Clt::<CltTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, None, name.clone()).unwrap();

    let id = format!("clt2clt_nonblocking_busywait_recv_msg SvcTestMsg");
    let mut clt_initiator_msg_recv_count = 0_usize;

    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator.recv_busywait().unwrap().unwrap();
                clt_initiator_msg_recv_count += 1;
            })
        })
    });

    drop(clt_initiator); // this will allow svc.join to complete
    let clt_acceptor_msg_send_count = clt_acceptor_jh.join().unwrap();
    info!("clt_acceptor_msg_send_count: {:?} > clt_initiator_msg_recv_count: {:?}", fmt_num!(clt_acceptor_msg_send_count), fmt_num!(clt_initiator_msg_recv_count));

    assert!(clt_acceptor_msg_send_count > clt_initiator_msg_recv_count);
}

fn round_trip_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<SvcTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::bind(addr, svc_callback, max_connections, None, name.clone()).unwrap();

            // info!("svc: {}", svc);

            let mut clt_acceptor = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted();
            info!("clt_acceptor: {}", clt_acceptor);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            let mut clt_acceptor_msg_send = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));
            loop {
                match clt_acceptor.recv_busywait() {
                    Ok(Some(_recv_msg)) => {
                        clt_acceptor.send_busywait(&mut clt_acceptor_msg_send).unwrap();
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(None) => {
                        info!("Connection Closed by clt_initiator clt_acceptor: {}", clt_acceptor);
                        break;
                    }
                    Err(err) => {
                        info!("Connection Closed by clt_initiator, clt_acceptor: {}, err: {}", clt_acceptor, err);
                        break;
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let mut clt_initiator = Clt::<CltTestProtocolAuth, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, None, name.clone()).unwrap();
    info!("clt_initiator: {}", clt_initiator);

    let id = format!("clt2clt_nonblocking_busywait_round_trip_msg SvcTestMsg");
    let mut clt_initiator_msg_send_count = 0_usize;
    let mut clt_initiator_send_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator.send_busywait(&mut clt_initiator_send_msg).unwrap();
                let _msg = clt_initiator.recv_busywait().unwrap().unwrap();
                clt_initiator_msg_send_count += 1;
            })
        })
    });

    drop(clt_initiator); // this will allow svc.join to complete
    let clt_acceptor_msg_recv_count = clt_acceptor_jh.join().unwrap();
    info!("clt_acceptor_msg_recv_count: {:?}, clt_initiator_msg_send_count: {:?}", fmt_num!(clt_acceptor_msg_recv_count), fmt_num!(clt_initiator_msg_send_count));
    assert_eq!(clt_initiator_msg_send_count, clt_acceptor_msg_recv_count);
}

criterion_group!(benches, send_msg, recv_msg, round_trip_msg);

criterion_main!(benches);
