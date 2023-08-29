use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::prelude::Framer;
use links_network_sync::connect::framer::nonblocking::{into_split_framer, Partial};
use links_testing::unittest::setup;
use log::{error, info};
use num_format::{Locale, ToFormattedString};

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
    let random_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, _) =
                    into_split_framer::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("svc: reader: {}", reader);
                let mut frame_recv_count = 0_u32;
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(Partial::Content(None)) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(Partial::Content(Some(_))) => {
                            frame_recv_count += 1;
                        }
                        Ok(Partial::NotReady) => {
                            continue; // try reading again
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

    // CONFIGUR clt
    let (_, mut writer) = into_split_framer::<BenchMsgFramer>(
        TcpStream::connect(addr).unwrap(),
        BENCH_MAX_FRAME_SIZE,
    );
    // info!("clt: writer: {}", writer);

    let id = format!(
        "send_random_frame NON-BLOCKING size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                writer.write_frame(random_frame).unwrap();
                frame_send_count += 1;
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    let frame_recv_count = reader.join().unwrap();
    info!(
        "send_count: {:?}, recv_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
    );

    assert_eq!(frame_send_count, frame_recv_count);
}

fn recv_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let random_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);
    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let writer = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (_, mut writer) =
                    into_split_framer::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("svc: writer: {}", writer);
                let mut frame_send_count = 0_u32;
                loop {
                    let res = writer.write_frame(random_frame);
                    match res {
                        Ok(_) => {}
                        Err(e) => {
                            info!("Svc write_frame, expected error: {}", e.to_string()); // not error as client will stop reading and drop
                            break;
                        }
                    }
                    frame_send_count += 1;
                }
                frame_send_count
            }
        })
        .unwrap();

    // CONFIGUR clt
    let (mut reader, _) = into_split_framer::<BenchMsgFramer>(
        TcpStream::connect(addr).unwrap(),
        BENCH_MAX_FRAME_SIZE,
    );
    // info!("clt: reader: {}", reader);

    let id = format!(
        "recv_random_frame NON-BLOCKING size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(Partial::Content(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }

                        Ok(Partial::NotReady) => {
                            continue;
                        }
                        _ => {
                            panic!("clt: read_frame error: {:?}", res);
                        }
                    }
                }
            })
        })
    });

    drop(reader); // this will allow svc.join to complete
    let frame_send_count = writer.join().unwrap();
    info!(
        "send_count: {:?}, recv_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
    );

    assert!(frame_send_count > frame_recv_count);
}

fn round_trip_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let random_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                stream.set_nodelay(true).unwrap();
                let (mut reader, mut writer) =
                    into_split_framer::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("svc: reader: {}", reader);
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(Partial::Content(None)) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(Partial::Content(Some(recv_frame))) => {
                            writer.write_frame(&recv_frame).unwrap();
                        }
                        Ok(Partial::NotReady) => {
                            continue; // try reading again
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

    // CONFIGUR clt
    let stream = TcpStream::connect(addr).unwrap();
    stream.set_nodelay(true).unwrap();
    let (mut reader, mut writer) =
        into_split_framer::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
    // info!("clt: writer: {}", writer);

    let id = format!(
        "round_trip_random_frame NON-BLOCKING size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_send_count = 0_u32;
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                writer.write_frame(random_frame).unwrap();
                frame_send_count += 1;
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(Partial::Content(None)) => {
                            panic!("clt: read_frame is None, server closed connection");
                        }
                        Ok(Partial::Content(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(Partial::NotReady) => {
                            continue;
                        }
                        Err(e) => {
                            panic!("clt: read_frame error: {}", e.to_string());
                        }
                    }
                }
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    drop(reader);
    svc.join().unwrap();
    info!(
        "send_count: {:?}, recv_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
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
