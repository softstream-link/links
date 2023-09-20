#![feature(tcp_linger)]

use std::io::Error;
use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};

const EOF: usize = 0;
fn read(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; 10];
    let n = stream.read(&mut buf)?;
    println!("recv: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn write(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; 10];
    let n = stream.write(&mut buf)?;
    println!("send: {:x?}, {:?}", &buf[..n], stream);
    Ok(n)
}

fn main() -> Result<(), Error> {
    clt2clt_noclone()
}

fn clt2clt_noclone() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;
    let mut clt = TcpStream::connect(addr)?;
    let (mut svc, _addr) = acp.accept()?;

    println!("clt: {:?}, svc: {:?}", clt, svc);

    clt.shutdown(std::net::Shutdown::Both)?;

    assert_eq!(read(&mut clt)?, EOF);

    // drop(clt);

    // pause_for_input("read post shutdown");
    let n = read(&mut svc)?;
    assert_eq!(n, EOF);

    // pause_for_input("write post shutdown");
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

// fn pause_for_input(message: &str) {
//     let mut buf = String::new();
//     let msg = format!("Press Enter to continue with {}...\n", message);
//     std::io::stdout()
//         .write_all(msg.as_str().as_bytes())
//         .unwrap();
//     std::io::stdin().read_line(&mut buf).unwrap();
// }

// pub fn netstat(message: &str) {
//     use std::process::{Command, Stdio};
//     use std::str;
//     let netstat = Command::new("netstat")
//         .arg("-nat")
//         .stdout(Stdio::piped())
//         .spawn()
//         .unwrap();
//     let grep = Command::new("grep")
//         .arg("8080")
//         .stdin(Stdio::from(netstat.stdout.unwrap()))
//         .stdout(Stdio::piped())
//         .spawn()
//         .expect("failed to execute process");

//     let output = grep.wait_with_output().unwrap();
//     let result = str::from_utf8(&output.stdout).unwrap();

//     println!("result: {}\n{}", message, result)
// }
