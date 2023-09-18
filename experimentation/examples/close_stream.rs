#![feature(tcp_linger)]

use std::io::Error;
use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};

const EOF: usize = 0;
fn read(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; 10];
    let n = stream.read(&mut buf)?;
    println!("recv: {:x?}", &buf[..n]);
    Ok(n)
}

fn write(stream: &mut TcpStream) -> Result<usize, Error> {
    let mut buf = [1; 10];
    let n = stream.write(&mut buf)?;
    println!("send: {:x?}", &buf[..n]);
    Ok(n)
}

fn main() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;
    let mut clt = TcpStream::connect(addr)?;
    let (mut svc_recv, _addr) = acp.accept()?;
    // let mut svc_send = svc_recv.try_clone()?;

    println!("clt: {:?}, svc: {:?}", clt, svc_recv);

    assert_ne!(write(&mut clt)?, EOF);
    assert_ne!(read(&mut svc_recv)?, EOF);

    // netstat("before clt shutdown");
    clt.shutdown(std::net::Shutdown::Write)?;
    // clt.close();
    // netstat("after clt shutdown");
    // println!("clt: {:?}, svc: {:?}", clt, svc);
    // drop(clt);
    // netstat("after clt drop");
    // drop(svc);

    // println!("svc: {:?}", svc);
    // Why does read immediatelly recognizes that client reset connection
    assert_eq!(read(&mut svc_recv)?, EOF); // pass - as expected - client disconnected
                                           // drop(svc);
                                           // svc_recv.shutdown(std::net::Shutdown::Write)?;
    assert_eq!(write(&mut svc_recv)?, EOF); // fail - NOT as expected - does not realize client disconnected

    netstat("final");

    Ok(())
}

fn netstat(message: &str) {
    use std::process::{Command, Stdio};
    use std::str;
    let netstat = Command::new("netstat")
        .arg("-nat")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let grep = Command::new("grep")
        .arg("8080")
        .stdin(Stdio::from(netstat.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to execute process");

    let output = grep.wait_with_output().unwrap();
    let result = str::from_utf8(&output.stdout).unwrap();

    println!("result: {}\n{}", message, result)
}
