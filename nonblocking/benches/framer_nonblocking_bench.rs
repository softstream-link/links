use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_core::{fmt_num, unittest::setup};
use links_nonblocking::prelude::{into_split_framer, ConId, FixedSizeFramer, RecvStatus, SendStatus};

use log::{error, info};
use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

const BENCH_MAX_FRAME_SIZE: usize = 128;
pub type BenchMsgFramer = FixedSizeFramer<BENCH_MAX_FRAME_SIZE>;

fn send_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc_reader_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, _writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::svc(Some("benchmark"), addr, None), stream);
                let mut frame_recv_count = 0_u32;
                loop {
                    match reader.read_frame() {
                        Ok(RecvStatus::Completed(None)) => {
                            info!("svc: read_frame is None, clt CLEAN connection close");
                            break;
                        }
                        Ok(RecvStatus::Completed(Some(recv_frame))) => {
                            frame_recv_count += 1;
                            assert_eq!(send_frame, &recv_frame[..]);
                            continue;
                        }
                        Ok(RecvStatus::WouldBlock) => {
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

    // CONFIGURE clt
    let (_clt_reader, mut clt_writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::clt(Some("benchmark"), None, addr), TcpStream::connect(addr).unwrap());
    // info!("clt: writer: {}", writer);

    let id = format!("framer_nonblocking_send_random_frame size: {} bytes", fmt_num!(BENCH_MAX_FRAME_SIZE));
    let mut frame_send_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match clt_writer.write_frame(send_frame) {
                        Ok(SendStatus::Completed) => {
                            frame_send_count += 1;
                            break;
                        }
                        Ok(SendStatus::WouldBlock) => {
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

    drop(clt_writer); // this will allow svc.join to complete
    let frame_recv_count = svc_reader_jh.join().unwrap();
    info!("frame_send_count: {:?} = frame_recv_count: {:?}", fmt_num!(frame_send_count), fmt_num!(frame_recv_count));

    assert_eq!(frame_send_count, frame_recv_count);
}

fn recv_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Debug);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);
    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc_writer_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (_reader, mut writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::svc(Some("benchmark"), addr, None), stream);
                // info!("svc: writer: {}", writer);
                let mut frame_send_count = 0_u32;
                loop {
                    match writer.write_frame(send_frame) {
                        Ok(SendStatus::Completed) => {
                            frame_send_count += 1;
                        }
                        Ok(SendStatus::WouldBlock) => {
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

    // CONFIGURE clt
    let (mut clt_reader, _clt_writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::clt(Some("benchmark"), None, addr), TcpStream::connect(addr).unwrap());
    // info!("clt: reader: {}", reader);

    let id = format!("framer_nonblocking_recv_random_frame size: {} bytes", fmt_num!(BENCH_MAX_FRAME_SIZE));
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match clt_reader.read_frame() {
                        Ok(RecvStatus::Completed(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(RecvStatus::WouldBlock) => {
                            continue;
                        }
                        Ok(RecvStatus::Completed(None)) => {
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

    drop(clt_reader); // this will allow svc.join to complete
    drop(_clt_writer); // TODO git hub issue - https://github.com/bheisler/criterion.rs/issues/726
    let frame_send_count = svc_writer_jh.join().unwrap();
    info!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count),
        fmt_num!(frame_send_count - frame_recv_count)
    );

    assert!(frame_send_count > frame_recv_count);
}

fn round_trip_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut svc_reader, mut svc_writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::svc(Some("benchmark"), addr, None), stream);
                // info!("svc: reader: {}", reader);
                loop {
                    let res = svc_reader.read_frame();
                    match res {
                        Ok(RecvStatus::Completed(None)) => {
                            info!("svc: read_frame is None, client closed connection");
                            break;
                        }
                        Ok(RecvStatus::Completed(Some(recv_frame))) => {
                            svc_writer.write_frame(&recv_frame).unwrap();
                        }
                        Ok(RecvStatus::WouldBlock) => {
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

    // CONFIGURE clt
    let stream = TcpStream::connect(addr).unwrap();
    let (mut clt_reader, mut clt_writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(ConId::clt(Some("benchmark"), None, addr), stream);
    // info!("clt: writer: {}", writer);

    let id = format!("framer_nonblocking_round_trip_random_frame size: {} bytes", fmt_num!(BENCH_MAX_FRAME_SIZE));
    let mut frame_send_count = 0_u32;
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                loop {
                    match clt_writer.write_frame(send_frame) {
                        Ok(SendStatus::Completed) => {
                            frame_send_count += 1;
                            break;
                        }
                        Ok(SendStatus::WouldBlock) => {
                            continue;
                        }
                        Err(e) => {
                            panic!("clt: write_frame error: {}", e.to_string());
                        }
                    }
                }
                loop {
                    match clt_reader.read_frame() {
                        Ok(RecvStatus::Completed(None)) => {
                            panic!("clt: read_frame is None, server closed connection");
                        }
                        Ok(RecvStatus::Completed(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(RecvStatus::WouldBlock) => {
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

    drop(clt_writer); // this will allow svc.join to complete
                      // drop(clt_reader);
    svc_jh.join().unwrap();
    info!("frame_send_count: {:?} = frame_recv_count: {:?}", fmt_num!(frame_send_count), fmt_num!(frame_recv_count));

    assert_eq!(frame_send_count, frame_recv_count);
}

criterion_group!(benches, send_random_frame, recv_random_frame, round_trip_random_frame);

criterion_main!(benches);
