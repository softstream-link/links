use std::{
    io::{Read, Write},
    net::TcpListener,
};

use links_testing::unittest::setup;
use log::info;

fn main() {
    setup::log::configure();
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).unwrap();
    info!("listening on {}", addr);

    let mut buf = [0u8; 1024];
    for stream in listener.incoming() {
        info!("accepted connection {:?}", stream);
        let mut stream = stream.unwrap();
        stream.set_nodelay(true).unwrap();
        loop {
            match stream.read(&mut buf) {
                Ok(0) => {
                    info!("connection closed {:?}", stream);
                    break;
                }
                Ok(len) => {
                    info!("read {} bytes {:?}", len, buf[..len].as_ref());
                    let _ = stream.write_all(buf[..len].as_ref()).unwrap();
                    stream.flush().unwrap();
                    continue;
                }
                Err(e) => {
                    info!("read error {:?}", e);
                    break;
                }
            }
        }
    }
}
