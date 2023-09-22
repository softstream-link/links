use std::io::Error;
use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;

const FRAME_SIZE: usize = 10;
const EOF: usize = 0;
fn read(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [0; FRAME_SIZE];
    let n = stream.read(&mut buf)?;
    println!("read: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn write(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; FRAME_SIZE];
    let n = stream.write(&mut buf)?;
    println!("write: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn main() -> Result<(), Error> {
    clt2svc()
}

fn clt2svc() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;
    let mut clt = TcpStream::connect(addr)?;
    let (mut svc, _addr) = acp.accept()?;

    println!("clt: {:?}, svc: {:?}", clt, svc);

    clt.shutdown(std::net::Shutdown::Both)?;

    assert_eq!(read(&mut clt)?, EOF);

    sleep(Duration::from_secs(1));

    let n = read(&mut svc)?;
    assert_eq!(n, EOF);

    let mut shutdown_detected_on_write_number = 0_usize;
    loop {
        match write(&mut svc) {
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
