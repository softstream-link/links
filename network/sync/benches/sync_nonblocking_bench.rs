use criterion::{black_box, criterion_group, criterion_main, Criterion};

use links_testing::unittest::setup;
use log::info;
use num_format::{Locale, ToFormattedString};
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
fn send_random_frame(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
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

    let id = format!(
        "sync_nonblocking_send_random_frame size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
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
    let id = format!(
        "sync_nonblocking_recv_random_frame size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );

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
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en),
        (frame_send_count - frame_recv_count).to_formatted_string(&Locale::en)
    );

    assert!(frame_send_count > frame_recv_count);
}

// fn round_trip_random_frame(c: &mut Criterion) {
//     setup::log::configure_level(log::LevelFilter::Info);
//     let send_frame = setup::data::random_bytes(BENCH_MAX_FRAME_SIZE);

//     let addr = setup::net::rand_avail_addr_port();

//     // CONFIGURE svc
//     let svc = thread::Builder::new()
//         .name("Thread-Svc".to_owned())
//         .spawn({
//             move || {
//                 let listener = TcpListener::bind(addr).unwrap();
//                 let (stream, _) = listener.accept().unwrap();
//                 let (mut svc_reader, mut svc_writer) =
//                     into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
//                         ConId::svc(Some("benchmark"), addr, None),
//                         stream,
//                     );
//                 // info!("svc: reader: {}", reader);
//                 loop {
//                     let res = svc_reader.read_frame();
//                     match res {
//                         Ok(RecvStatus::Completed(None)) => {
//                             info!("svc: read_frame is None, client closed connection");
//                             break;
//                         }
//                         Ok(RecvStatus::Completed(Some(recv_frame))) => {
//                             svc_writer.write_frame(&recv_frame).unwrap();
//                         }
//                         Ok(RecvStatus::WouldBlock) => {
//                             continue; // try reading again
//                         }
//                         Err(e) => {
//                             error!("Svc read_frame error: {}", e.to_string());
//                             break;
//                         }
//                     }
//                 }
//             }
//         })
//         .unwrap();

//     sleep(Duration::from_millis(100)); // allow the spawned to bind

//     // CONFIGUR clt
//     let stream = TcpStream::connect(addr).unwrap();
//     let (mut clt_reader, mut clt_writer) = into_split_framer::<BenchMsgFramer, BENCH_MAX_FRAME_SIZE>(
//         ConId::clt(Some("benchmark"), None, addr),
//         stream,
//     );
//     // info!("clt: writer: {}", writer);

//     let id = format!(
//         "framer_sync_nonblocking_round_trip_random_frame size: {} bytes",
//         BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
//     );
//     let mut frame_send_count = 0_u32;
//     let mut frame_recv_count = 0_u32;
//     c.bench_function(id.as_str(), |b| {
//         b.iter(|| {
//             black_box({
//                 loop {
//                     match clt_writer.write_frame(send_frame) {
//                         Ok(SendStatus::Completed) => {
//                             frame_send_count += 1;
//                             break;
//                         }
//                         Ok(SendStatus::WouldBlock) => {
//                             continue;
//                         }
//                         Err(e) => {
//                             panic!("clt: write_frame error: {}", e.to_string());
//                         }
//                     }
//                 }
//                 loop {
//                     match clt_reader.read_frame() {
//                         Ok(RecvStatus::Completed(None)) => {
//                             panic!("clt: read_frame is None, server closed connection");
//                         }
//                         Ok(RecvStatus::Completed(Some(_))) => {
//                             frame_recv_count += 1;
//                             break;
//                         }
//                         Ok(RecvStatus::WouldBlock) => {
//                             continue;
//                         }
//                         Err(e) => {
//                             panic!("clt: read_frame error: {}", e.to_string());
//                         }
//                     }
//                 }
//             })
//         })
//     });

//     drop(clt_writer); // this will allow svc.join to complete
//                       // drop(clt_reader);
//     svc.join().unwrap();
//     info!(
//         "frame_send_count: {:?} = frame_recv_count: {:?}",
//         frame_send_count.to_formatted_string(&Locale::en),
//         frame_recv_count.to_formatted_string(&Locale::en)
//     );

//     assert_eq!(frame_send_count, frame_recv_count);
// }

criterion_group!(
    benches,
    send_random_frame,
    recv_random_frame,
    // round_trip_random_frame
);
// criterion_group!(benches, recv_random_frame);
criterion_main!(benches);
