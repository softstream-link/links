use std::io::{Error, ErrorKind};
use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};
use std::thread::Builder;

use criterion::{black_box, Criterion};

const EOF: usize = 0;
const FRAME_SIZE: usize = 128;

fn read_busywait(stream: &mut TcpStream) -> Result<(), Error> {
    let mut buf = [0; FRAME_SIZE];
    let mut filled = 0_usize;

    loop {
        match stream.read(&mut buf[filled..]) {
            Ok(EOF) => {
                return Err(Error::new(ErrorKind::NotConnected, "stream.read EOF"));
                // END of stream no more reads possible
            }
            Ok(n) => {
                filled += n;
                if filled == FRAME_SIZE {
                    return Ok(()); // entire frame read return
                } else {
                    continue;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

fn write_busywait(stream: &mut TcpStream) -> Result<(), Error> {
    let mut buf = [1; FRAME_SIZE];
    let mut written = 0_usize;
    let mut would_block_count = 0_usize;
    loop {
        match stream.write(&mut buf[written..]) {
            Ok(EOF) => {
                return Err(Error::new(ErrorKind::NotConnected, "streamw.write got EOF"));
                // END of stream no more writes possible
            }
            Ok(n) => {
                written += n;
                if written == FRAME_SIZE {
                    return Ok(());
                } else {
                    continue;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                would_block_count += 1;
                if would_block_count % 10_000 == 0{
                    println!("write_busywait: would_block_count: {}, written: {}, remaining: {}", would_block_count, written, FRAME_SIZE - written );
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

fn main() -> Result<(), Error> {
    let _ = clt2svc_without_criterion();
    let _ = clt2svc_with_criterion();
    Ok(())
}

fn clt2svc_with_criterion() -> Result<(), Error> {
    let mut c = Criterion::default();
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;

    let mut clt = TcpStream::connect(addr)?;

    clt.set_nonblocking(true)?;
    println!("clt: {:?}", clt);

    let mut write_frame_count = 0_usize;
    let svc_jh = Builder::new()
        .name("Svc-Thread".to_owned())
        .spawn(move || {
            let (mut svc, _addr) = acp.accept().unwrap();
            svc.set_nonblocking(true).unwrap();
            println!("svc: {:?}", svc);

            loop {
                match write_busywait(&mut svc) {
                    Ok(()) => {
                        write_frame_count += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("write error: {:?}", e);
                        break;
                    }
                }
            }

            write_frame_count
        })
        .unwrap();

    let mut read_frame_count = 0_usize;

    c.bench_function("read", |b| {
        b.iter(|| {
            black_box({
                match read_busywait(&mut clt) {
                    Ok(()) => {
                        read_frame_count += 1;
                    }
                    Err(e) => panic!("read error: {:?}", e),
                }
            })
        })
    });
    c.final_summary();

    clt.shutdown(std::net::Shutdown::Both)?;
    println!("read_frame_count: {}", read_frame_count);
    println!("wait for join on svc_jh - START");
    let write_frame_count = svc_jh.join().unwrap();
    println!("wait for join on svc_jh - DONE");
    println!("write_frame_count: {}", write_frame_count);
    Ok(())
}

fn clt2svc_without_criterion() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;

    let mut clt = TcpStream::connect(addr)?;

    clt.set_nonblocking(true)?;
    println!("clt: {:?}", clt);

    let mut write_frame_count = 0_usize;
    let svc_jh = Builder::new()
        .name("Svc-Thread".to_owned())
        .spawn(move || {
            let (mut svc, _addr) = acp.accept().unwrap();
            svc.set_nonblocking(true).unwrap();
            println!("svc: {:?}", svc);

            loop {
                match write_busywait(&mut svc) {
                    Ok(()) => {
                        write_frame_count += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("write error: {:?}", e);
                        break;
                    }
                }
            }

            write_frame_count
        })
        .unwrap();

    let mut read_frame_count = 0_usize;
    for _ in 0..10_000_000 {
        match read_busywait(&mut clt) {
            Ok(()) => {
                read_frame_count += 1;
            }
            Err(e) => panic!("read error: {:?}", e),
        }
    }

    clt.shutdown(std::net::Shutdown::Both)?;
    let write_frame_count = svc_jh.join().unwrap();
    println!("read_frame_count: {}", read_frame_count);
    println!("write_frame_count: {}", write_frame_count);
    Ok(())
}
