use std::{
    sync::mpsc::{channel, sync_channel, TryRecvError},
    thread::{self, sleep},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use links_testing::unittest::setup;
use log::info;
use num_format::{Locale, ToFormattedString};

fn send_random_frame_as_channel(c: &mut Criterion) {
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

    // CONFIGUR clt
    let id = format!("send_random_frame_as_channel size: {} bytes", 128);
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
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
    );

    assert!(frame_send_count > frame_recv_count);
}

fn send_random_frame_as_sync_channel(c: &mut Criterion) {
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

    // CONFIGUR clt
    let id = format!("send_random_frame_as_sync_channel size: {} bytes", 128);
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
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
    );

    assert_eq!(frame_send_count , frame_recv_count);
}

criterion_group!(
    benches,
    send_random_frame_as_channel,
    send_random_frame_as_sync_channel
);

criterion_main!(benches);