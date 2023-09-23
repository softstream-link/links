use std::{
    net::{TcpListener, TcpStream},
    // os::fd::AsRawFd,
    thread::{self, sleep},
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::{
    fmt_num,
    prelude::{ConId, Framer},
};
use links_network_sync::connect::framer::blocking::into_split_framer;
use links_testing::unittest::setup;
use log::{error, info};
use nix::sys::socket::{setsockopt, sockopt::ReusePort};

const BENCH_MAX_FRAME_SIZE: usize = 128;
pub struct BenchMsgFramer;
impl Framer for BenchMsgFramer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        if bytes.len() < BENCH_MAX_FRAME_SIZE {
            return None;
        } else {
            let frame = bytes.split_to(BENCH_MAX_FRAME_SIZE);
            return Some(frame.freeze());
        }
    }
}

fn send_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                setsockopt(&listener, ReusePort, &true).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, _) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
                    ConId::svc(Some("bench"), addr, None),
                    stream,
                );
                // info!("svc: reader: {}", reader);
                let mut frame_recv_count = 0_u32;
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(None) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(Some(recv_frame)) => {
                            frame_recv_count += 1;
                            assert_eq!(send_frame, recv_frame);
                        }
                        Err(e) => {
                            info!("Svc read_rame error: {}", e.to_string());
                            break;
                        }
                    }
                }
                frame_recv_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (_, mut writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
        ConId::clt(Some("bench"), None, addr),
        TcpStream::connect(addr).unwrap(),
    );
    // info!("clt: writer: {}", writer);

    let id = format!(
        "framer_blocking_send_random_frame size: {} bytes",
        fmt_num!(BENCH_MAX_FRAME_SIZE)
    );
    let mut frame_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                writer.write_frame(send_frame).unwrap();
                frame_send_count += 1;
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    let frame_recv_count = reader.join().unwrap();
    info!(
        "frame_send_count: {:?} = frame_recv_count: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count)
    );

    assert_eq!(frame_send_count, frame_recv_count);
}

fn recv_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);
    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let writer = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                setsockopt(&listener, ReusePort, &true).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (_, mut writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
                    ConId::svc(Some("bench"), addr, None),
                    stream,
                );
                // info!("svc: writer: {}", writer);
                let mut frame_send_count = 0_u32;
                loop {
                    let res = writer.write_frame(send_frame);
                    match res {
                        Ok(()) => {
                            frame_send_count += 1;
                        }
                        Err(e) => {
                            info!("Svc write_frame, expected error: {}", e.to_string()); // not error as client will stop reading and drop
                            break;
                        }
                    }
                }
                frame_send_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (mut reader, _) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
        ConId::clt(Some("bench"), None, addr),
        TcpStream::connect(addr).unwrap(),
    );
    // info!("clt: reader: {}", reader);

    let id = format!(
        "framer_blocking_recv_random_frame size: {} bytes",
        fmt_num!(BENCH_MAX_FRAME_SIZE)
    );
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                reader.read_frame().unwrap();
                frame_recv_count += 1;
            })
        })
    });

    drop(reader); // this will allow svc.join to complete
    let frame_send_count = writer.join().unwrap();
    info!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count),
        fmt_num!(frame_send_count - frame_recv_count),
    );

    assert!(frame_send_count > frame_recv_count);
}

fn round_trip_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, mut writer) =
                    into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
                        ConId::svc(Some("bench"), addr, None),
                        stream,
                    );
                // info!("svc: reader: {}", reader);
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(None) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(Some(recv_frame)) => {
                            writer.write_frame(&recv_frame).unwrap();
                        }
                        Err(e) => {
                            error!("Svc read_frame error: {}", e.to_string());
                            break;
                        }
                    }
                }
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut reader, mut writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
        ConId::clt(Some("bench"), None, addr),
        stream,
    );
    // info!("clt: writer: {}", writer);

    let id = format!(
        "framer_blocking_round_trip_random_frame size: {} bytes",
        fmt_num!(BENCH_MAX_FRAME_SIZE)
    );
    let mut frame_send_count = 0_u32;
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                writer.write_frame(send_frame).unwrap();
                frame_send_count += 1;
                let res = reader.read_frame();
                match res {
                    Ok(None) => {
                        panic!("clt: read_frame is None, server closed connection");
                    }
                    Ok(Some(_)) => {
                        frame_recv_count += 1;
                    }
                    Err(e) => {
                        panic!("clt: read_frame error: {}", e.to_string());
                    }
                }
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    drop(reader);
    svc.join().unwrap();
    info!(
        "frame_send_count: {:?} = frame_recv_count: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count)
    );

    assert_eq!(frame_send_count, frame_recv_count);
}

criterion_group!(
    benches,
    send_random_frame,
    recv_random_frame,
    round_trip_random_frame
);
// criterion_group!(benches, recv_random_frame);
criterion_main!(benches);
