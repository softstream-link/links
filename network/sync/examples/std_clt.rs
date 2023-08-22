use std::io::Read;
use std::io::Write;
use std::thread::spawn;
use std::{net::TcpStream, thread::sleep, time::Duration};

use links_testing::unittest::setup;
use log::info;
fn main() {
    setup::log::configure();
    let addr = "0.0.0.0:8080";
    let mut reader = TcpStream::connect(addr).unwrap();
    let mut writer = reader.try_clone().unwrap();
    reader.set_nonblocking(true).unwrap();
    println!("connected reader: {:?}", reader);
    println!("connected writer: {:?}", writer);
    spawn(move || {
        for i in 0..10 {
            loop {
                match writer.write(b"Hello World") {
                    Ok(0) => {
                        info!("connection closed {:?}", writer);
                        break;
                    }
                    Ok(len) => {
                        info!("write {} bytes {:?}", len, b"Hello World");
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        info!("write error {:?}", e);
                    }
                    Err(e) => {
                        info!("write error {:?}", e);
                        break;
                    }
                };
            }
            // sleep(Duration::from_secs(1));
        }
        // writer.write(b"Hello World").unwrap();
    });

    let reader_h = spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    info!("connection closed {:?}", reader);
                    break;
                },
                Ok(len) => {
                    info!("read {} bytes {:?}", len, buf[..len].as_ref());
                    continue;
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    info!("read error {:?}", e);
                    sleep(Duration::from_secs(1));
                    continue;
                },
                Err(e) => {
                    info!("read error {:?}", e);
                    break;
                }
            }
        }
    });
    // writer.write(b"Hello World").unwrap();
    reader_h.join().unwrap();
    // sleep(Duration::from_secs(1));
}
