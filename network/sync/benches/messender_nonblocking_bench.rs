use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::{prelude::ConId, fmt_num};
use links_network_sync::{
    prelude_nonblocking::{into_split_messenger, RecvMsgNonBlocking, RecvStatus, SendStatus},
    unittest::setup::{
        framer::TEST_MSG_FRAME_SIZE,
        messenger::{TestCltMsgProtocol, TestSvcMsgProtocol},
    },
};
use links_testing::unittest::setup::{
    self,
    model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
};
use log::info;
use nix::sys::socket::{setsockopt, sockopt::ReusePort};

fn send_msg(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn(move || {
            let listener = TcpListener::bind(addr).unwrap();
            setsockopt(&listener, ReusePort, &true).unwrap(); // TODO remove i don't think this is necessary
            let (stream, _) = listener.accept().unwrap();
            let (mut svc_reader, _svc_writer) =
                into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                    ConId::svc(Some("unittest"), addr, None),
                    stream,
                );
            // info!("svc: reader: {}", reader);
            let mut frame_recv_count = 0_u32;
            loop {
                let status = svc_reader.recv_nonblocking().unwrap();
                match status {
                    RecvStatus::Completed(Some(_)) => {
                        frame_recv_count += 1;
                    }
                    RecvStatus::Completed(None) => {
                        info!("svc: read_frame is None, client closed connection");
                        break;
                    }
                    RecvStatus::WouldBlock => continue,
                }
            }
            frame_recv_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (_clt_reader, mut clt_writer) =
        into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
            ConId::clt(Some("unittest"), None, addr),
            TcpStream::connect(addr).unwrap(),
        );
    // info!("clt: writer: {}", writer);

    let id = format!("messenger_nonblocking_send_msg TestCltMsg");

    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    let mut msg_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let SendStatus::WouldBlock = clt_writer.send_nonblocking(&msg).unwrap() {}
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
    let writer_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn(move || {
            let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
            let listener = TcpListener::bind(addr).unwrap();
            setsockopt(&listener, ReusePort, &true).unwrap();
            let (stream, _) = listener.accept().unwrap();
            let (_clt_reader, mut svc_writer) = into_split_messenger::<
                TestSvcMsgProtocol,
                TEST_MSG_FRAME_SIZE,
            >(ConId::svc(None, addr, None), stream);
            // info!("svc: writer: {}", writer);
            let mut msg_send_count = 0_u32;
            while let Ok(status) = svc_writer.send_nonblocking(&msg) {
                match status {
                    SendStatus::WouldBlock => continue,
                    SendStatus::Completed => {
                        msg_send_count += 1;
                    }
                }
            }
            msg_send_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (mut clt_reader, _clt_writer) =
        into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
            ConId::clt(Some("unittest"), None, addr),
            TcpStream::connect(addr).unwrap(),
        );
    // info!("clt: reader: {}", reader);

    let id = format!("messenger_nonblocking_recv_msg TestSvcMsg");
    let mut msg_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let RecvStatus::WouldBlock = clt_reader.recv_nonblocking().unwrap() {}
                msg_recv_count += 1;
            })
        })
    });

    drop(clt_reader); // this will allow svc.join to complete
    drop(_clt_writer); // TODO git hub issue - https://github.com/bheisler/criterion.rs/issues/726

    let msg_send_count = writer_jh.join().unwrap();
    info!(
        "msg_send_count: {:?} > msg_recv_count: {:?}, diff: {:?}",
        fmt_num!(msg_send_count),
        fmt_num!(msg_recv_count),
        fmt_num!(msg_send_count - msg_recv_count),
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
                let (mut reader, mut writer) =
                    into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        ConId::svc(Some("unittest"), addr, None),
                        stream,
                    );
                // info!("svc: reader: {}", reader);
                while let Ok(status) = reader.recv_nonblocking() {
                    match status {
                        RecvStatus::Completed(Some(_msg)) => {
                            while let SendStatus::WouldBlock =
                                writer.send_nonblocking(&msg).unwrap()
                            {}
                        }
                        RecvStatus::Completed(None) => {
                            info!("{} Connection Closed by Client", reader);
                            break;
                        }
                        RecvStatus::WouldBlock => continue,
                    }
                }
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut reader, mut writer) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        ConId::clt(Some("unittest"), None, addr),
        stream,
    );
    // info!("clt: writer: {}", writer);

    let id = format!("messenger_nonblocking_round_trip_msg",);
    let mut msg_send_count = 0_u32;
    let mut msg_recv_count = 0_u32;
    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let SendStatus::WouldBlock = writer.send_nonblocking(&msg).unwrap() {}
                msg_send_count += 1;

                loop {
                    match reader.recv_nonblocking().unwrap() {
                        RecvStatus::Completed(Some(_msg)) => {
                            msg_recv_count += 1;
                            break;
                        }
                        RecvStatus::Completed(None) => {
                            panic!("{} Connection Closed by Server", reader);
                            // break;
                        }
                        RecvStatus::WouldBlock => continue,
                    }
                }
            })
        })
    });

    drop(writer); // this will allow svc.join to complete

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
