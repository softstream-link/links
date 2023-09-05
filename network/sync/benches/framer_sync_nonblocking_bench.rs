use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::prelude::{Framer, ReadStatus, WriteStatus};
use links_network_sync::connect::framer::nonblocking::into_split_framer;
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
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, _) =
                    into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(stream);
                let mut frame_recv_count = 0_u32;
                loop {
                    match reader.read_frame() {
                        Ok(ReadStatus::Completed(None)) => {
                            info!("svc: read_frame is None, clt CLEAN connection close");
                            break;
                        }
                        Ok(ReadStatus::Completed(Some(recv_frame))) => {
                            frame_recv_count += 1;
                            // info!("svc: read_frame: {:?}, frame_recv_count: {}", &recv_frame[..], frame_recv_count);
                            assert_eq!(send_frame, &recv_frame[..]);
                            continue;
                        }
                        Ok(ReadStatus::NotReady) => {
                            // info!("svc: read_frame Not Ready {}", frame_recv_count);
                            continue; // try reading again
                        }
                        Err(e) => {
                            info!("Svc read_frame, expected error: {}", e);
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
        TcpStream::connect(addr).unwrap(),
    );
    // info!("clt: writer: {}", writer);

    let id = format!(
        "send_random_frame_as_sync_non-blocking size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match writer.write_frame(send_frame) {
                        Ok(WriteStatus::Completed) => {
                            frame_send_count += 1;
                            break;
                        }
                        Ok(WriteStatus::NotReady) => {
                            continue;
                        }
                        Err(e) => {
                            panic!("clt: write_frame error: {}", e);
                        }
                    }
                }
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    let frame_recv_count = reader.join().unwrap();
    info!(
        "frame_send_count: {:?} = frame_recv_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
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
                let (stream, _) = listener.accept().unwrap();
                let (_, mut writer) =
                    into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(stream);
                // info!("svc: writer: {}", writer);
                let mut frame_send_count = 0_u32;
                loop {
                    match writer.write_frame(send_frame) {
                        Ok(WriteStatus::Completed) => {
                            frame_send_count += 1;
                        }
                        Ok(WriteStatus::NotReady) => {
                            continue;
                        }
                        Err(e) => {
                            info!("Svc write_frame, expected error: {}", e); // not error as client will stop reading and drop
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
        TcpStream::connect(addr).unwrap(),
    );
    // info!("clt: reader: {}", reader);

    let id = format!(
        "recv_random_frame_as_sync_non-blocking size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match reader.read_frame() {
                        Ok(ReadStatus::Completed(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(ReadStatus::NotReady) => {
                            continue;
                        }
                        Ok(ReadStatus::Completed(None)) => {
                            panic!("clt: read_frame is None, server closed connection");
                        }
                        Err(e) => {
                            panic!("clt: read_frame error: {:?}", e);
                        }
                    }
                }
            })
        })
    });

    drop(reader); // this will allow svc.join to complete
    let frame_send_count = writer.join().unwrap();
    info!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en),
        (frame_send_count - frame_recv_count).to_formatted_string(&Locale::en)
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
                    into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(stream);
                // info!("svc: reader: {}", reader);
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(ReadStatus::Completed(None)) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(ReadStatus::Completed(Some(recv_frame))) => {
                            writer.write_frame(&recv_frame).unwrap();
                        }
                        Ok(ReadStatus::NotReady) => {
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

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut reader, mut writer) =
        into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(stream);
    // info!("clt: writer: {}", writer);

    let id = format!(
        "round_trip_random_frame_as_sync_non-blocking size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    let mut frame_send_count = 0_u32;
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match writer.write_frame(send_frame) {
                        Ok(WriteStatus::Completed) => {
                            frame_send_count += 1;
                            break;
                        }
                        Ok(WriteStatus::NotReady) => {
                            continue;
                        }
                        Err(e) => {
                            panic!("clt: write_frame error: {}", e.to_string());
                        }
                    }
                }
                loop {
                    match reader.read_frame() {
                        Ok(ReadStatus::Completed(None)) => {
                            panic!("clt: read_frame is None, server closed connection");
                        }
                        Ok(ReadStatus::Completed(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(ReadStatus::NotReady) => {
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
        "frame_send_count: {:?} = frame_recv_count: {:?}",
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
