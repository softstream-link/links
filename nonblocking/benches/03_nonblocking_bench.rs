use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_core::{fmt_num, unittest::setup};
use log::{info, LevelFilter};
use std::io::ErrorKind;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

const BENCH_MAX_FRAME_SIZE: usize = 128;
pub struct BenchMsgFramer;

const EOF: usize = 0;
static LOG_LEVEL: LevelFilter = LevelFilter::Error;

fn send_random_frame(c: &mut Criterion) {
    setup::log::configure_level(LOG_LEVEL);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc_reader = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (mut svc, _) = listener.accept().unwrap();
                svc.set_nonblocking(true).unwrap();
                info!("svc: {:?}", svc);
                let mut byte_recv_count = 0;
                loop {
                    let mut buf = [0_u8; BENCH_MAX_FRAME_SIZE];
                    match svc.read(buf.as_mut_slice()) {
                        Ok(EOF) => {
                            info!("svc: read_frame is None, clt CLEAN connection close");
                            break;
                        }
                        Ok(n) => {
                            byte_recv_count += n;
                            continue;
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            continue; // try reading again
                        }
                        Err(e) => {
                            info!("Svc read_frame, expected error: {}", e);
                            break;
                        }
                    }
                }
                byte_recv_count / BENCH_MAX_FRAME_SIZE
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let mut clt = TcpStream::connect(addr).unwrap();
    clt.set_nonblocking(true).unwrap();
    info!("clt: {:?}", clt);

    let id = format!("nonblocking_send_random_frame size: {} bytes", fmt_num!(BENCH_MAX_FRAME_SIZE));
    let mut frame_send_count = 0_usize;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let mut residual = send_frame;
                while !residual.is_empty() {
                    match clt.write(residual) {
                        Ok(0) => {
                            panic!("write, connection closed");
                        }
                        Ok(n) => {
                            residual = &residual[n..];
                            continue;
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            continue; // try writing again
                        }
                        Err(e) => {
                            panic!("write, unexpected error: {}", e);
                        }
                    }
                }
                frame_send_count += 1;
            })
        })
    });

    drop(clt); // this will allow svc.join to complete
    let frame_recv_count = svc_reader.join().unwrap();
    info!("frame_send_count: {:?} = frame_recv_count: {:?}", fmt_num!(frame_send_count), fmt_num!(frame_recv_count));

    assert_eq!(frame_send_count, frame_recv_count);
}

fn recv_random_frame(c: &mut Criterion) {
    setup::log::configure_level(LOG_LEVEL);
    let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);
    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc_writer = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (mut svc, _) = listener.accept().unwrap();
                svc.set_nonblocking(true).unwrap();
                info!("svc: {:?}", svc);
                let mut bytes_send_count = 0_usize;
                let mut done_writing = false;
                loop {
                    let mut residual = send_frame;
                    while !residual.is_empty() {
                        match svc.write(residual) {
                            Ok(0) => {
                                info!("write, connection closed, clean");
                                done_writing = true;
                                break;
                            }
                            Ok(n) => {
                                bytes_send_count += n;
                                residual = &residual[n..];
                                continue;
                            }
                            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                                continue; // try writing again
                            }
                            Err(e) => {
                                info!("write, connection closed, error: {}", e);
                                done_writing = true;
                                break;
                            }
                        }
                    }
                    if done_writing {
                        return bytes_send_count / BENCH_MAX_FRAME_SIZE;
                    }
                }
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let mut clt = TcpStream::connect(addr).unwrap();
    clt.set_nonblocking(true).unwrap();
    info!("clt: {:?}", clt);

    sleep(Duration::from_millis(1000));
    let id = format!("nonblocking_recv_random_frame size: {} bytes", fmt_num!(BENCH_MAX_FRAME_SIZE));

    let mut byte_recv_count = 0;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box(loop {
                let mut buf = [0_u8; BENCH_MAX_FRAME_SIZE];
                match clt.read(buf.as_mut_slice()) {
                    Ok(0) => {
                        info!("read_frame is None, clt CLEAN connection close");
                        break;
                    }
                    Ok(n) => {
                        byte_recv_count += n;
                        if byte_recv_count % BENCH_MAX_FRAME_SIZE == 0 {
                            break;
                        }
                        continue;
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        continue; // try reading again
                    }
                    Err(e) => {
                        info!("read_frame, expected error: {}", e);
                        break;
                    }
                }
            })
        })
    });
    let frame_recv_count = byte_recv_count / BENCH_MAX_FRAME_SIZE;

    drop(clt); // this will allow svc.join to complete
    let frame_send_count = svc_writer.join().unwrap();
    info!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count),
        fmt_num!(frame_send_count - frame_recv_count)
    );

    assert!(frame_send_count > frame_recv_count);
}

criterion_group!(benches, send_random_frame, recv_random_frame);

criterion_main!(benches);
