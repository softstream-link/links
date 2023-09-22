use std::io::{Error, ErrorKind};
use std::io::{Read, Write};

use mio::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;

const EOF: usize = 0;
const FRAME_SIZE: usize = 128;

fn read_busywait(stream: &mut TcpStream) -> Result<usize, Error> {
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
    println!("read_busywait: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn write_busywait(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; FRAME_SIZE];
    let mut written = 0_usize;
    let n = loop {
        match stream.write(&mut buf[written..]) {
            Ok(EOF) => {
                break written; // END of stream no more writes possible
            }
            Ok(n) => {
                written += n;
                if written == FRAME_SIZE {
                    break written;
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

    println!("write_busywait: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn main() -> Result<(), Error> {
    clt2svc()
}

fn clt2svc() -> Result<(), Error> {
    let addr = "0.0.0.0:8080".parse().unwrap();
    let acp = TcpListener::bind(addr)?;
    sleep(Duration::from_secs(1));
    let mut clt = TcpStream::connect(addr)?;
    sleep(Duration::from_secs(1));
    let (mut svc, _addr) = acp.accept()?;

    println!("clt: {:?}, svc: {:?}", clt, svc);

    clt.shutdown(std::net::Shutdown::Both)?;

    assert_eq!(read_busywait(&mut clt)?, EOF);

    sleep(Duration::from_secs(1));

    let n = read_busywait(&mut svc)?;
    assert_eq!(n, EOF);

    let mut shutdown_detected_on_write_number = 0_usize;
    loop {
        match write_busywait(&mut svc) {
            Ok(n) => {
                if n == EOF {
                    println!("write incomplete n: {}", n);
                    break;
                } else {
                    shutdown_detected_on_write_number += 1;
                    continue;
                }
            }
            Err(e) => {
                println!("write error: {:?}", e);
                break;
            }
        }
    }
    println!(
        "shutdown_detected_on_write_number: {}",
        shutdown_detected_on_write_number
    );

    Ok(())
}
