use std::{
    net::{TcpListener, TcpStream},
    thread,
};

use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_core::prelude::Framer;
use links_network_sync::connect::framer::blocking::into_split_frame_manager;
use links_testing::unittest::setup;
use log::{error, info};
use num_format::{Locale, ToFormattedString};

const BENCH_SEND_FRAME_SIZE: usize = 128;
pub struct MsgFramer;
impl Framer for MsgFramer {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        if bytes.len() < BENCH_SEND_FRAME_SIZE {
            return None;
        } else {
            let frame = bytes.split_to(BENCH_SEND_FRAME_SIZE);
            return Some(frame.freeze());
        }
    }
}

fn frame_send(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let sending_frame: [u8; BENCH_SEND_FRAME_SIZE] = (0..BENCH_SEND_FRAME_SIZE)
        .map(|_| rand::random::<u8>())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    let svc = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut reader, _) = into_split_frame_manager::<MsgFramer>(stream, BENCH_SEND_FRAME_SIZE);
                info!("svc: reader: {}", reader);
                let mut frame_recv_count = 0_u32;
                loop {
                    let res = reader.read_frame();
                    match res {
                        Ok(frame) => {
                            if let None = frame {
                                info!("svc: read_frame is None, client closed connection");
                                break;
                            } else {
                                frame_recv_count += 1;
                            }
                        }
                        Err(e) => {
                            error!("Svc read_rame error: {}", e.to_string());
                            break;
                        }
                    }
                }
                frame_recv_count
            }
        })
        .unwrap();

    let (_, mut clt) = into_split_frame_manager::<MsgFramer>(TcpStream::connect(addr).unwrap(), BENCH_SEND_FRAME_SIZE);

    // CONFIGUR clt
    info!(
        "random benchmark send_frame: \n{}",
        to_hex_pretty(&sending_frame)
    );
    let mut frame_send_count = 0_u32;
    c.bench_function("sending_frame", |b| {
        b.iter(|| {
            black_box({
                clt.write_frame(&sending_frame).unwrap();
                frame_send_count += 1;
            })
        })
    });

    drop(clt);
    let frame_recv_count = svc.join().unwrap();

    info!(
        "frame_send_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en)
    );
    info!(
        "frame_recv_count: {:?}",
        frame_recv_count.to_formatted_string(&Locale::en)
    );
    assert_eq!(frame_send_count, frame_recv_count);
}

criterion_group!(benches, frame_send);
criterion_main!(benches);
