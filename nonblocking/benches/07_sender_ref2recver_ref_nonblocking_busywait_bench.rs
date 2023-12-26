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
    unittest::setup::protocol::{CltTestProtocolManual, SvcTestProtocolManual},
};
use log::{info, LevelFilter};
static LOG_LEVEL: LevelFilter = LevelFilter::Error;

fn setup<MSvc: Messenger, MClt: Messenger>() -> (&'static str, Arc<impl CallbackRecvSend<MSvc>>, Arc<impl CallbackRecvSend<MClt>>, NonZeroUsize, Option<&'static str>, Duration, Duration) {
    setup::log::configure_level(LOG_LEVEL);
    let addr = setup::net::rand_avail_addr_port();
    let svc_callback = DevNullCallback::<MSvc>::new_ref();
    let clt_callback = DevNullCallback::<MClt>::new_ref();
    let name = Some("bench");
    let max_connections = NonZeroUsize::new(1).unwrap();
    let timeout = setup::net::default_connect_timeout();
    let retry_after = timeout / 10;
    (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after)
}

fn send_msg(c: &mut Criterion) {
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, max_connections, svc_callback, SvcTestProtocolManual::default(), name.clone()).unwrap();

            // info!("svc: {}", svc);

            let (mut clt_acceptor_recv, mut _clt_acceptor_send) = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted().into_split_ref();
            info!("clt_acceptor_recv: {}", clt_acceptor_recv);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            loop {
                match clt_acceptor_recv.recv() {
                    Ok(RecvStatus::Completed(Some(_recv_msg))) => {
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(RecvStatus::Completed(None)) => {
                        info!("Connection Closed by clt_initiator clt_acceptor_recv: {}", clt_acceptor_recv);
                        break;
                    }
                    Ok(RecvStatus::WouldBlock) => continue,
                    Err(err) => {
                        panic!("Connection Closed by clt_initiator, clt_acceptor_recv: {}, err: {}", clt_acceptor_recv, err);
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let (mut _clt_initiator_recv, mut clt_initiator_send) = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, CltTestProtocolManual::default(), name.clone())
        .unwrap()
        .into_split_ref();
    info!("clt_initiator_send: {}", clt_initiator_send);

    let id = format!("sender_ref2recver_ref_nonblocking_busywait_send_msg SvcTestMsg");
    let mut clt_initiator_msg_send_count = 0_usize;
    let mut clt_initiator_send_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator_send.send_busywait(&mut clt_initiator_send_msg).unwrap();
                clt_initiator_msg_send_count += 1;
            })
        })
    });

    drop(clt_initiator_send); // this will allow svc.join to complete
    let clt_acceptor_msg_recv_count = clt_acceptor_jh.join().unwrap();
    info!("clt_acceptor_msg_recv_count: {:?}, clt_initiator_msg_send_count: {:?}", fmt_num!(clt_acceptor_msg_recv_count), fmt_num!(clt_initiator_msg_send_count),);
    assert_eq!(clt_acceptor_msg_recv_count, clt_initiator_msg_send_count);
}

fn recv_msg(c: &mut Criterion) {
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    // CONFIGURE svc
    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, max_connections, svc_callback, SvcTestProtocolManual::default(), name.clone()).unwrap();

            let (mut _clt_acceptor_recv, mut clt_acceptor_send) = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted().into_split_ref();
            info!("clt_acceptor_send: {}", clt_acceptor_send);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            let mut clt_acceptor_msg_send = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));
            loop {
                match clt_acceptor_send.send(&mut clt_acceptor_msg_send) {
                    Ok(SendStatus::Completed) => {
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(SendStatus::WouldBlock) => continue,
                    Err(err) => {
                        info!("Connection Closed by clt_initiator, clt_acceptor_send: {}, err: {}", clt_acceptor_send, err);
                        break;
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let (mut clt_initiator_recv, mut _clt_initiator_send) = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, CltTestProtocolManual::default(), name.clone())
        .unwrap()
        .into_split_ref();

    let id = format!("sender_ref2recver_ref_nonblocking_busywait_recv_msg SvcTestMsg");
    let mut clt_initiator_msg_recv_count = 0_usize;

    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator_recv.recv_busywait().unwrap().unwrap();
                clt_initiator_msg_recv_count += 1;
            })
        })
    });

    drop(clt_initiator_recv); // this will allow svc.join to complete
    drop(_clt_initiator_send); // TODO git hub issue - https://github.com/bheisler/criterion.rs/issues/726
    let clt_acceptor_msg_send_count = clt_acceptor_jh.join().unwrap();
    info!(
        "clt_acceptor_msg_send_count: {:?} > clt_initiator_msg_recv_count: {:?}, diff: {:?}",
        fmt_num!(clt_acceptor_msg_send_count),
        fmt_num!(clt_initiator_msg_recv_count),
        fmt_num!(clt_acceptor_msg_send_count - clt_initiator_msg_recv_count)
    );

    assert!(clt_acceptor_msg_send_count > clt_initiator_msg_recv_count);
}

fn round_trip_msg(c: &mut Criterion) {
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, max_connections, svc_callback, SvcTestProtocolManual::default(), name.clone()).unwrap();

            let (mut clt_acceptor_recv, mut clt_acceptor_send) = svc.accept_busywait_timeout(timeout).unwrap().unwrap_accepted().into_split_ref();
            info!("clt_acceptor_recv: {}", clt_acceptor_recv);

            let mut clt_acceptor_msg_recv_count = 0_usize;
            let mut clt_acceptor_msg_send = SvcTestMsg::Dbg(SvcTestMsgDebug::new(b"Hello Frm Server Msg"));
            loop {
                match clt_acceptor_recv.recv_busywait() {
                    Ok(Some(_recv_msg)) => {
                        clt_acceptor_send.send_busywait(&mut clt_acceptor_msg_send).unwrap();
                        clt_acceptor_msg_recv_count += 1;
                    }
                    Ok(None) => {
                        info!("Connection Closed by clt_initiator clt_acceptor_recv: {}", clt_acceptor_recv);
                        break;
                    }
                    Err(err) => {
                        info!("Connection Closed by clt_initiator, clt_acceptor_recv: {}, err: {}", clt_acceptor_recv, err);
                        break;
                    }
                }
            }
            clt_acceptor_msg_recv_count
        })
        .unwrap();

    let (mut clt_initiator_recv, mut clt_initiator_send) = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, timeout, retry_after, clt_callback, CltTestProtocolManual::default(), name.clone())
        .unwrap()
        .into_split_ref();
    info!("clt_initiator_recv: {}", clt_initiator_recv);

    let id = format!("sender_ref2recver_ref_nonblocking_busywait_round_trip_msg SvcTestMsg");
    let mut clt_initiator_msg_send_count = 0_usize;
    let mut clt_initiator_send_msg = CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_initiator_send.send_busywait(&mut clt_initiator_send_msg).unwrap();
                let _msg = clt_initiator_recv.recv_busywait().unwrap().unwrap();
                clt_initiator_msg_send_count += 1;
            })
        })
    });

    drop(clt_initiator_send); // this will allow svc.join to complete
    let clt_acceptor_msg_recv_count = clt_acceptor_jh.join().unwrap();
    info!(
        "clt_acceptor_msg_recv_count: {:?} > clt_initiator_msg_send_count: {:?}",
        fmt_num!(clt_acceptor_msg_recv_count),
        fmt_num!(clt_initiator_msg_send_count)
    );

    assert_eq!(clt_initiator_msg_send_count, clt_acceptor_msg_recv_count);
}

criterion_group!(benches, send_msg, recv_msg, round_trip_msg);

criterion_main!(benches);
