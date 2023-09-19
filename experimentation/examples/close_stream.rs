#![feature(tcp_linger)]

use std::io::Error;
use std::io::{Read, Write};

use std::net::{TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;

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
    clt2clt_noclone()
}

fn clt2clt_noclone() -> Result<(), Error> {
    let addr = "0.0.0.0:8080";
    let acp = TcpListener::bind(addr)?;
    let mut clt = TcpStream::connect(addr)?;
    let (mut svc, _addr) = acp.accept()?;
    // 6	2.015543	127.0.0.1	127.0.0.1	TCP	68	52608 → 8080 [SYN] Seq=0 Win=65535 Len=0 MSS=16344 WS=64 TSval=3532670732 TSecr=0 SACK_PERM
    // 7	2.015728	127.0.0.1	127.0.0.1	TCP	68	8080 → 52608 [SYN, ACK] Seq=0 Ack=1 Win=65535 Len=0 MSS=16344 WS=64 TSval=1236383785 TSecr=3532670732 SACK_PERM
    // 8	2.015760	127.0.0.1	127.0.0.1	TCP	56	52608 → 8080 [ACK] Seq=1 Ack=1 Win=408256 Len=0 TSval=3532670732 TSecr=1236383785
    // 9	2.015779	127.0.0.1	127.0.0.1	TCP	56	[TCP Window Update] 8080 → 52608 [ACK] Seq=1 Ack=1 Win=408256 Len=0 TSval=1236383785 TSecr=3532670732

    println!("clt: {:?}, svc: {:?}", clt, svc);
    pause_for_input("read/write");

    for _ in 0..1 {
        pause_for_input("clt write");
        assert_ne!(write(&mut clt)?, EOF);
        // 12	37.415851	127.0.0.1	127.0.0.1	TCP	66	52608 → 8080 [PSH, ACK] Seq=1 Ack=1 Win=408256 Len=10 TSval=3532706133 TSecr=1236383785
        // 13	37.415922	127.0.0.1	127.0.0.1	TCP	56	8080 → 52608 [ACK] Seq=1 Ack=11 Win=408256 Len=0 TSval=1236419186 TSecr=3532706133

        pause_for_input("svc read");
        assert_ne!(read(&mut svc)?, EOF);
        sleep(Duration::from_secs(1));
    }

    pause_for_input("Shutdown::Read");
    clt.shutdown(std::net::Shutdown::Read)?;
    // ********* NOTHING ***********

    assert_eq!(read(&mut clt)?, EOF);
    // pause_for_input("Shutdown::Write");
    // clt.shutdown(std::net::Shutdown::Write)?;
    // // 19	65.944818	127.0.0.1	127.0.0.1	TCP	56	52608 → 8080 [FIN, ACK] Seq=11 Ack=1 Win=408256 Len=0 TSval=3532734662 TSecr=1236419186
    // // 20	65.944870	127.0.0.1	127.0.0.1	TCP	56	8080 → 52608 [ACK] Seq=1 Ack=12 Win=408256 Len=0 TSval=1236447715 TSecr=3532734662

    pause_for_input("drop");
    drop(clt);
    // 12	29.893297	127.0.0.1	127.0.0.1	TCP	56	52696 → 8080 [FIN, ACK] Seq=11 Ack=1 Win=408256 Len=0 TSval=2470608228 TSecr=2527546041
    // 13	29.893353	127.0.0.1	127.0.0.1	TCP	56	8080 → 52696 [ACK] Seq=1 Ack=12 Win=408256 Len=0 TSval=2527567796 TSecr=2470608228

    pause_for_input("read post shutdown");
    let n = read(&mut svc)?;
    // ********* NOTHING ***********

    pause_for_input("write post shutdown");
    assert_eq!(n, EOF);

    let _n = write(&mut svc)?;
    // IF only WRITE is SHUTDOWN
    // 15	16.456955	127.0.0.1	127.0.0.1	TCP	66	8080 → 52634 [PSH, ACK] Seq=1 Ack=12 Win=408256 Len=10 TSval=3502553995 TSecr=307104660
    // 16	16.457039	127.0.0.1	127.0.0.1	TCP	56	52634 → 8080 [ACK] Seq=12 Ack=11 Win=408256 Len=0 TSval=307117156 TSecr=3502553995
    // IF both WRITE & READ SHUTDOWN
    // 17	67.450044	127.0.0.1	127.0.0.1	TCP	66	8080 → 52643 [PSH, ACK] Seq=1 Ack=12 Win=408256 Len=10 TSval=2927182 TSecr=3227935204
    // 18	67.450125	127.0.0.1	127.0.0.1	TCP	44	52643 → 8080 [RST] Seq=12 Win=0 Len=0

    pause_for_input("write 2 post shutdown");

    let _n = write(&mut svc)?;
    // assert_eq!(n, EOF);

    pause_for_input("exit");

    Ok(())
}

fn pause_for_input(message: &str) {
    let mut buf = String::new();
    let msg = format!("Press Enter to continue with {}...\n", message);
    std::io::stdout()
        .write_all(msg.as_str().as_bytes())
        .unwrap();
    std::io::stdin().read_line(&mut buf).unwrap();
}

pub fn netstat(message: &str) {
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
