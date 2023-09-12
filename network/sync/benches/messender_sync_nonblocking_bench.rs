use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::prelude::ConId;
use links_network_sync::{
    prelude_nonblocking::{
        into_split_messenger, ReadStatus, RecvMsgNonBlocking, SendMsgNonBlocking, WriteStatus,
    },
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
use num_format::{Locale, ToFormattedString};

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
            let (mut reader, _) = into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                stream,
                ConId::svc(Some("unittest"), addr, None),
            );
            // info!("svc: reader: {}", reader);
            let mut frame_recv_count = 0_u32;
            loop {
                let status = reader.recv_nonblocking().unwrap();
                match status {
                    ReadStatus::Completed(Some(_)) => {
                        frame_recv_count += 1;
                    }
                    ReadStatus::Completed(None) => {
                        info!("svc: read_frame is None, client closed connection");
                        break;
                    }
                    ReadStatus::WouldBlock => continue,
                }
            }
            frame_recv_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (_, mut writer) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        TcpStream::connect(addr).unwrap(),
        ConId::clt(Some("unittest"), None, addr),
    );
    // info!("clt: writer: {}", writer);

    let id = format!("send_msg_as_sync_non-blocking TestCltMsg");

    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    let mut msg_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let WriteStatus::WouldBlock = writer.send_nonblocking(&msg).unwrap() {}
                msg_send_count += 1;
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    let msg_recv_count = reader.join().unwrap();
    info!(
        "msg_send_count: {:?}, msg_recv_count: {:?}",
        msg_send_count.to_formatted_string(&Locale::en),
        msg_recv_count.to_formatted_string(&Locale::en)
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
            setsockopt(&listener, ReusePort, &true).unwrap();
            let (stream, _) = listener.accept().unwrap();
            let (_, mut writer) = into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                stream,
                ConId::svc(None, addr, None),
            );
            // info!("svc: writer: {}", writer);
            let mut frame_send_count = 0_u32;
            while let Ok(status) = writer.send_nonblocking(&msg) {
                match status {
                    WriteStatus::WouldBlock => continue,
                    WriteStatus::Completed => {
                        frame_send_count += 1;
                    }
                }
            }
            frame_send_count
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (mut reader, _) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        TcpStream::connect(addr).unwrap(),
        ConId::clt(Some("unittest"), None, addr),
    );
    // info!("clt: reader: {}", reader);

    let id = format!("recv_msg_as_sync_non-blocking TestSvcMsg");
    let mut msg_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let ReadStatus::WouldBlock = reader.recv_nonblocking().unwrap() {}
                msg_recv_count += 1;
            })
        })
    });

    drop(reader); // this will allow svc.join to complete
    let msg_send_count = writer.join().unwrap();
    info!(
        "msg_send_count: {:?} > msg_recv_count: {:?}",
        msg_send_count.to_formatted_string(&Locale::en),
        msg_recv_count.to_formatted_string(&Locale::en)
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
                        stream,
                        ConId::svc(Some("unittest"), addr, None),
                    );
                // info!("svc: reader: {}", reader);
                while let Ok(status) = reader.recv_nonblocking() {
                    match status {
                        ReadStatus::Completed(Some(_msg)) => {
                            while let WriteStatus::WouldBlock = writer.send_nonblocking(&msg).unwrap() {}
                        }
                        ReadStatus::Completed(None) => {
                            info!("{} Connection Closed by Client", reader);
                            break;
                        }
                        ReadStatus::WouldBlock => continue,
                    }
                }
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut reader, mut writer) = into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
        stream,
        ConId::clt(Some("unittest"), None, addr),
    );
    // info!("clt: writer: {}", writer);

    let id = format!("round_trip_msg_as_sync_non-blocking",);
    let mut msg_send_count = 0_u32;
    let mut msg_recv_count = 0_u32;
    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                while let WriteStatus::WouldBlock = writer.send_nonblocking(&msg).unwrap() {}
                msg_send_count += 1;

                loop {
                    match reader.recv_nonblocking().unwrap() {
                        ReadStatus::Completed(Some(_msg)) => {
                            msg_recv_count += 1;
                            break;
                        }
                        ReadStatus::Completed(None) => {
                            panic!("{} Connection Closed by Server", reader);
                            // break;
                        }
                        ReadStatus::WouldBlock => continue,
                    }
                }
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    drop(reader);
    svc.join().unwrap();
    info!(
        "msg_send_count: {:?}, msg_recv_count: {:?}",
        msg_send_count.to_formatted_string(&Locale::en),
        msg_recv_count.to_formatted_string(&Locale::en)
    );

    assert_eq!(msg_send_count, msg_recv_count);
}

criterion_group!(benches, send_msg, recv_msg, round_trip_msg);

criterion_main!(benches);