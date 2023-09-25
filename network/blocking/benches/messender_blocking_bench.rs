use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_blocking::{
    prelude::*,
    unittest::setup::messenger::TestCltMsgProtocol,
    unittest::setup::{framer::TEST_MSG_FRAME_SIZE, messenger::TestSvcMsgProtocol},
};
use links_network_core::{fmt_num, prelude::ConId};
use links_testing::unittest::setup::{
    self,
    model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
};
use log::{error, info};

fn send_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn(move || {
            let listener = TcpListener::bind(addr).unwrap();
            let (stream, _) = listener.accept().unwrap();
            let (mut svc_reader, _svc_writer) =
                into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                    ConId::svc(Some("unittest"), addr, None),
                    stream,
                );
            // info!("svc: reader: {}", reader);
            let mut frame_recv_count = 0_u32;
            while let Some(_) = svc_reader.recv().unwrap() {
                frame_recv_count += 1;
            }
            info!("svc: {} Client Closed Connection", svc_reader);
            frame_recv_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGURE clt
    let (_clt_reader, mut clt_writer) =
        into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
            ConId::clt(Some("unittest"), None, addr),
            TcpStream::connect(addr).unwrap(),
        );
    // info!("clt: writer: {}", writer);

    let id = format!("messenger_blocking_send_msg TestCltMsg");

    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    let mut msg_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_writer.send(&msg).unwrap();
                msg_send_count += 1;
            })
        })
    });

    drop(clt_writer); // this will allow svc.join to complete
    let msg_recv_count = reader.join().unwrap();
    info!(
        "msg_send_count: {:?}, msg_recv_count: {:?}",
        fmt_num!(msg_send_count),
        fmt_num!(msg_recv_count)
    );
    assert_eq!(msg_send_count, msg_recv_count);
}

fn recv_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let writer = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn(move || {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
            let listener = TcpListener::bind(addr).unwrap();
            let (stream, _) = listener.accept().unwrap();
            let (_svc_reader, mut svc_writer) = into_split_messenger::<
                TestSvcMsgProtocol,
                TEST_MSG_FRAME_SIZE,
            >(ConId::svc(None, addr, None), stream);
            // info!("svc: writer: {}", writer);
            let mut frame_send_count = 0_u32;
            while let Ok(_) = svc_writer.send(&msg) {
                frame_send_count += 1;
            }
            frame_send_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGURE clt
    let (mut clt_reader, _clt_writer) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        ConId::clt(Some("unittest"), None, addr),
        TcpStream::connect(addr).unwrap(),
    );
    // info!("clt: reader: {}", reader);

    let id = format!("messenger_blocking_recv_msg TestSvcMsg");
    let mut msg_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let _x = clt_reader.recv().unwrap();
                msg_recv_count += 1;
            })
        })
    });

    drop(clt_reader); // this will allow svc.join to complete
    let msg_send_count = writer.join().unwrap();
    info!(
        "msg_send_count: {:?}, msg_recv_count: {:?}",
        fmt_num!(msg_send_count),
        fmt_num!(msg_recv_count)
    );

    assert!(msg_send_count > msg_recv_count);
}

fn round_trip_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut svc_reader, mut svc_writer) =
                    into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        ConId::svc(Some("unittest"), addr, None),
                        stream,
                    );
                // info!("svc: reader: {}", reader);
                loop {
                    let res = svc_reader.recv();
                    match res {
                        Ok(None) => {
                            info!("svc: recv is None, client closed connection");
                            break;
                        }
                        Ok(Some(_)) => {
                            svc_writer.send(&msg).unwrap();
                        }
                        Err(e) => {
                            error!("Svc recv error: {}", e.to_string());
                            break;
                        }
                    }
                }
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGURE clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut clt_reader, mut clt_writer) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        ConId::clt(Some("unittest"), None, addr),
        stream,
    );
    // info!("clt: writer: {}", writer);

    let id = format!("messenger_blocking_round_trip_msg",);
    let mut msg_send_count = 0_u32;
    let mut msg_recv_count = 0_u32;
    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_writer.send(&msg).unwrap();
                msg_send_count += 1;

                match clt_reader.recv().unwrap() {
                    None => {
                        panic!("{} Server Closed Connection", clt_reader);
                    }
                    Some(_) => {
                        msg_recv_count += 1;
                    }
                }
            })
        })
    });

    drop(clt_writer); // this will allow svc.join to complete
    drop(clt_reader);
    svc.join().unwrap();
    info!(
        "msg_send_count: {:?}, msg_recv_count: {:?}",
        fmt_num!(msg_send_count),
        fmt_num!(msg_recv_count)
    );

    assert_eq!(msg_send_count, msg_recv_count);
}

criterion_group!(benches, send_msg, recv_msg, round_trip_msg);
criterion_main!(benches);
