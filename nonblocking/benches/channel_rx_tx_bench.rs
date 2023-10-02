use std::{
    sync::mpsc::{channel, sync_channel, TryRecvError},
    thread::{self, sleep},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use links_core::{fmt_num, unittest::setup};
use log::info;

fn channel_rx_tx_send_random_frame(c: &mut Criterion) {
    setup::log::configure();

    let (tx, rx) = channel::<&[u8]>();
    // CONFIGURE svc
    let sender = thread::Builder::new()
        .name("Thread-Sender".to_owned())
        .spawn({
            move || {
                let send_frame = setup::data::random_bytes(128);
                let mut frame_send_count = 0_u32;
                loop {
                    let res = tx.send(send_frame);
                    match res {
                        Ok(_) => frame_send_count += 1,
                        Err(_) => break,
                    }
                }
                frame_send_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGURE clt
    let id = format!("channel_rx_tx_send_random_frame size: {} bytes", 128);
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let res = rx.try_recv();
                match res {
                    Ok(_) => frame_recv_count += 1,
                    Err(TryRecvError::Empty) => {}
                    Err(_) => panic!("rx.recv() failed"),
                }
            })
        })
    });

    drop(rx); // this will allow svc.join to complete
    let frame_send_count = sender.join().unwrap();

    info!(
        "frame_send_count: {:?} > frame_recv_count: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count)
    );

    assert!(frame_send_count > frame_recv_count);
}

fn channel_rx_tx_send_random_frame_sync(c: &mut Criterion) {
    setup::log::configure();

    let (tx, rx) = sync_channel::<&[u8]>(0);
    // CONFIGURE svc
    let sender = thread::Builder::new()
        .name("Thread-Sender".to_owned())
        .spawn({
            move || {
                let send_frame = setup::data::random_bytes(128);
                let mut frame_send_count = 0_u32;
                loop {
                    let res = tx.send(send_frame);
                    match res {
                        Ok(_) => frame_send_count += 1,
                        Err(_) => break,
                    }
                }
                frame_send_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGURE clt
    let id = format!("channel_rx_tx_send_random_frame_sync size: {} bytes", 128);
    let mut frame_recv_count = 0_u32;
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                let res = rx.try_recv();
                match res {
                    Ok(_) => frame_recv_count += 1,
                    Err(TryRecvError::Empty) => {}
                    Err(_) => panic!("rx.recv() failed"),
                }
            })
        })
    });

    drop(rx); // this will allow svc.join to complete
    let frame_send_count = sender.join().unwrap();

    info!(
        "frame_send_count: {:?} = frame_recv_count: {:?}",
        fmt_num!(frame_send_count),
        fmt_num!(frame_recv_count)
    );

    assert_eq!(frame_send_count, frame_recv_count);
}

criterion_group!(
    benches,
    channel_rx_tx_send_random_frame,
    channel_rx_tx_send_random_frame_sync
);

criterion_main!(benches);
