use std::io::{Error, ErrorKind};
use std::io::{Read, Write};

use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

const EOF: usize = 0;
const FRAME_SIZE: usize = 128;

fn read_frame(stream: &mut mio::net::TcpStream) -> Result<usize, Error> {
    let mut buf = [0; FRAME_SIZE];
    let mut filled = 0_usize;

    let n = loop {
        match stream.read(&mut buf[filled..]) {
            Ok(EOF) => {
                break EOF; // END of stream no more reads possible
            }
            Ok(n) => {
                filled += n;
                if filled == FRAME_SIZE {
                    break filled;
                } else {
                    continue;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => return Err(e),
        }
    };
    // println!("read_busywait: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}
pub enum SendStatus {
    Completed,
    WouldBlock,
}
fn write_frame(stream: &mut mio::net::TcpStream, bytes: &[u8]) -> Result<SendStatus, Error> {
    let mut residual = bytes;
    while !residual.is_empty() {
        match stream.write(residual) {
            // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
            Ok(EOF) => {
                stream.shutdown(Shutdown::Both); // remember to shutdown on both exception and on EOF
                let msg = format!("connection reset by peer, residual buf:\n{:?}", residual);
                return Err(Error::new(ErrorKind::ConnectionReset, msg));
            }
            Ok(len) => {
                if len == residual.len() {
                    return Ok(SendStatus::Completed);
                } else {
                    residual = &residual[len..];
                    continue;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if bytes.len() == residual.len() {
                    // no bytes where written so Just report back NotReady
                    return Ok(SendStatus::WouldBlock);
                } else {
                    // some bytes where written have to finish and report back Completed or Error
                    continue;
                }
            }
            Err(e) => {
                stream.shutdown(Shutdown::Both); // remember to shutdown on both exception and on EOF
                let msg = format!(
                    "writer_frame caused by: [{}], residual len: {}\n{:?}",
                    e,
                    residual.len(),
                    residual
                );
                return Err(Error::new(e.kind(), msg));
            }
        }
    }
    Ok(SendStatus::Completed)

    // println!("write_busywait: {:x?}, {:?}", &buf[..n], stream);
}

fn main() -> Result<(), Error> {
    clt2svc()
}

pub fn into_split_mio(stream: TcpStream) -> (mio::net::TcpStream, mio::net::TcpStream) {
    stream
        .set_nonblocking(true)
        .expect("Failed to set_nonblocking on TcpStream");

    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for FrameReader"),
        stream,
    );

    (
        mio::net::TcpStream::from_std(reader),
        mio::net::TcpStream::from_std(writer),
    )
}

fn clt2svc() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;
    let clt = std::net::TcpStream::connect(addr)?;

    let (mut clt_read, mut _clt_write) = into_split_mio(clt);
    println!("clt_read: {:?}, clt_write: {:?}", clt_read, _clt_write);

    let svc_jh = spawn(move || {
        sleep(Duration::from_secs(1)); // wait clt to bind
        let (svc, _addr) = acp.accept().unwrap();
        let (mut reader, mut writer) = into_split_mio(svc);

        println!("svc_read: {:?}, svc_write: {:?}", reader, writer);

        let send_frame = [1_u8; FRAME_SIZE].as_slice();
        let mut svc_write_count = 0_usize;
        loop {
            match write_frame(&mut writer, send_frame) {
                Ok(SendStatus::Completed) => {
                    svc_write_count += 1;
                }
                Ok(SendStatus::WouldBlock) => {
                    continue;
                }
                Err(e) => {
                    println!("Svc write_frame, expected error: {}", e); // not error as client will stop reading and drop
                    break;
                }
            }
        }
        // println!("svc_write_count: {}", svc_write_count);

        svc_write_count
    });

    const clt_read_count: usize = 10_000_000;
    let now = Instant::now();
    for _ in 0..clt_read_count {
        let x = read_frame(&mut clt_read)?;
    }
    let elapsed = now.elapsed();

    println!("clt read finished");
    clt_read.shutdown(std::net::Shutdown::Both)?;
    println!("clt read shutdown BOTH finished");
    // drop(clt_read);
    let svc_write_count = svc_jh.join().unwrap();
    println!("svc_write_count: {}", svc_write_count);

    println!(
        "svc_write_count: {:?} > clt_read_count: {:?}, diff: {:?}",
        svc_write_count,
        clt_read_count,
        svc_write_count - clt_read_count,
    );
    println!(
        "elapsed per read: {:?}, elapsed total: {:?}",
        elapsed / clt_read_count as u32,
        elapsed,
    );

    // assert_eq!(read_busywait(&mut clt_read)?, EOF);
    // assert_eq!(
    //     write_busywait(&mut _clt_write).unwrap_err().kind(),
    //     ErrorKind::BrokenPipe
    // );

    // sleep(Duration::from_secs(1));

    // let n = read_busywait(&mut _svc_read).unwrap();
    // assert_eq!(n, EOF);

    Ok(())
}
