use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Builder,
};

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_network_async::connect::framer::into_split_frame_manager;
use links_network_core::prelude::Framer;
use links_testing::unittest::setup::{self, data::random_bytes};
use log::info;
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

fn send_random_frame_block_on(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_runtime = Builder::new_multi_thread().enable_all().build().unwrap();
    let clt_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc

    let svc = svc_runtime.spawn(async move {
        let listener = TcpListener::bind(addr).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let (mut reader, _) =
            into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
        // info!("svc: reader: {}", reader);
        let mut frame_recv_count = 0_u32;
        loop {
            let res = reader.read_frame().await;
            match res {
                Ok(None) => {
                    info!("svc: read_frame is None, client closed connection");
                    break;
                }
                Ok(Some(_)) => {
                    frame_recv_count += 1;
                }
                Err(e) => {
                    info!("Svc read_rame error: {}", e.to_string());
                    break;
                }
            }
        }
        frame_recv_count
    });

    // CONFIGUR clt
    let mut writer = clt_runtime.block_on(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        let (_, writer) = into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
        info!("clt: writer: {}", writer);
        writer
    });

    let mut frame_send_count = 0_u32;
    let random_frame = random_bytes(BENCH_MAX_FRAME_SIZE);
    let id = format!(
        "send_random_frame_block_on size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    c.bench_function(id.as_str(), |b| {
        b.iter(|| {
            black_box({
                clt_runtime.block_on(async { writer.write_frame(random_frame).await.unwrap() });
                frame_send_count += 1;
            })
        })
    });

    drop(writer); // this will allow svc.join to complete
    let frame_recv_count = svc_runtime.block_on(async move { svc.await.unwrap() });
    info!(
        "send_count: {:?}, recv_count: {:?}",
        frame_send_count.to_formatted_string(&Locale::en),
        frame_recv_count.to_formatted_string(&Locale::en)
    );

    assert_eq!(frame_send_count, frame_recv_count);
}

fn send_random_frame_as_async(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    svc_runtime.spawn(async move {
        let listener = TcpListener::bind(addr).await.unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut reader, _) =
                into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
            // info!("svc: reader: {}", reader);
            loop {
                let res = reader.read_frame().await;
                match res {
                    Ok(None) => {
                        // info!("svc: read_frame is None, client closed connection");
                        break;
                    }
                    Ok(Some(_)) => {}
                    Err(e) => {
                        info!("Svc read_rame error: {}", e.to_string());
                        break;
                    }
                }
            }
        }
    });

    let random_frame = random_bytes(BENCH_MAX_FRAME_SIZE);
    let id = format!(
        "send_random_frame_as_async size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    c.bench_function(id.as_str(), {
        |b| {
            let clt_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

            b.to_async(clt_runtime).iter_custom(|n| async move {
                let stream = TcpStream::connect(addr).await.unwrap();
                let (_, mut writer) =
                    into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("clt: writer: {}", n);

                let start = std::time::Instant::now();
                for _ in 0..n {
                    let _ = writer.write_frame(random_frame).await.unwrap();
                }
                start.elapsed()
            });
        }
    });
}

fn recv_random_frame_as_async(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    let addr = setup::net::rand_avail_addr_port();

    let random_frame = random_bytes(BENCH_MAX_FRAME_SIZE);

    // CONFIGURE svc
    svc_runtime.spawn(async move {
        let listener = TcpListener::bind(addr).await.unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (_, mut writer) =
                into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
            // info!("svc: reader: {}", reader);
            loop {
                let res = writer.write_frame(random_frame).await;
                match res {
                    Ok(()) => {}
                    Err(_) => {
                        // info!("Svc read_rame error: {}", e.to_string());  // this is normal during benchmark
                        break;
                    }
                }
            }
        }
    });

    let id = format!(
        "recv_random_frame_as_async size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    c.bench_function(id.as_str(), {
        |b| {
            let clt_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

            b.to_async(clt_runtime).iter_custom(|n| async move {
                let stream = TcpStream::connect(addr).await.unwrap();
                let (mut reader, _) =
                    into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("clt: reader: {}", n);

                let start = std::time::Instant::now();
                for _ in 0..n {
                    let _ = reader.read_frame().await.unwrap().unwrap();
                }
                start.elapsed()
            });
        }
    });
}
fn round_trip_random_frame_as_async(c: &mut Criterion) {
    setup::log::configure_level(log::LevelFilter::Info);
    let svc_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

    let addr = setup::net::rand_avail_addr_port();

    // CONFIGURE svc
    svc_runtime.spawn(async move {
        let listener = TcpListener::bind(addr).await.unwrap();
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut reader, mut writer) =
                into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
            // info!("svc: reader: {}", reader);
            loop {
                let res = reader.read_frame().await;
                match res {
                    Ok(None) => {
                        // info!("svc: read_frame is None, client closed connection"); // this is normal during benchmark
                        break;
                    }
                    Ok(Some(frame)) => {
                        writer.write_frame(&frame).await.unwrap();
                    }
                    Err(e) => {
                        info!("Svc read_rame error: {}", e.to_string());
                        break;
                    }
                }
            }
        }
    });

    let random_frame = random_bytes(BENCH_MAX_FRAME_SIZE);
    let id = format!(
        "round_trip_random_frame_as_async size: {} bytes",
        BENCH_MAX_FRAME_SIZE.to_formatted_string(&Locale::en)
    );
    c.bench_function(id.as_str(), {
        |b| {
            let clt_runtime = Builder::new_multi_thread().enable_all().build().unwrap();

            b.to_async(clt_runtime).iter_custom(|n| async move {
                let stream = TcpStream::connect(addr).await.unwrap();
                let (mut reader, mut writer) =
                    into_split_frame_manager::<BenchMsgFramer>(stream, BENCH_MAX_FRAME_SIZE);
                // info!("clt: writer: {}", n);

                let start = std::time::Instant::now();
                for _ in 0..n {
                    let _ = writer.write_frame(random_frame).await.unwrap();
                    let _ = reader.read_frame().await.unwrap().unwrap();
                }
                start.elapsed()
            });
        }
    });
}

criterion_group!(
    benches,
    recv_random_frame_as_async,
    send_random_frame_as_async,
    round_trip_random_frame_as_async,
    send_random_frame_block_on,
);

criterion_main!(benches);
