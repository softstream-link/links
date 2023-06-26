use std::{
    io::Read,
    net::TcpStream,
    sync::{
        Arc,
    },
    thread::{sleep, Builder},
    time::{Duration, Instant},
};

use log::info;

use super::callback::ReadCallback;

#[derive(Debug)]
pub struct Clt<CALLBACK>
where
    CALLBACK: ReadCallback,
{
    addr: String,
    retry_interval: Duration,
    stream: Option<Arc<TcpStream>>,
    callback: Arc<CALLBACK>,
    // exit_connect_thread: Arc<AtomicBool>,
    // jh_connect: JoinHandle<()>,
}

// impl<T: ReadCallback> Drop for Clt<T> {
//     fn drop(&mut self) {
//         self.exit_connect_thread.store(true, Ordering::SeqCst);
//         self.jh_connect.join().unwrap();

//         info!("dropping Clt");

//     }
// }
impl<T: ReadCallback> Clt<T> {
    pub fn connected(&self) -> bool {
        self.stream.is_some()
    }

    pub fn new_with_timeout(
        addr: &str,
        callback: Arc<T>,
        timeout: Duration,
        retry_interval: Duration,
        block_untill_connected: bool,
    ) -> Result<Clt<T>, std::io::Error> {
        // let exiting = Arc::new(AtomicBool::new(false));

        // let exit_connect = Arc::clone(&exiting);
        let spawn_name = format!("Reader {}", addr);
        let spawn_addr = addr.to_owned();
        let spawn_callback = Arc::clone(&callback);
        let jh_connect = Builder::new()
            .name(spawn_name)
            .spawn(move || {
                // let cb = spawn_callback;
                while true {
                    let mut stream_reader = TcpStream::connect(spawn_addr.clone()).expect("Failed to connect");
                    let  mut buf = [0_u8; 1024];
                    while true {
                        let n = stream_reader.read(&mut buf).expect("socket read failed");

                    }

                }
                info!("Connector loop finished")
            })
            .unwrap();

        let clt = Clt {
            addr: addr.to_owned(),
            retry_interval,
            stream: None,
            callback,
        };

        if block_untill_connected {
            let start = Instant::now();
            while start.elapsed() < timeout {
                if clt.connected() {
                    return Ok(clt);
                } else {
                    info!(
                        "Connecting... timout: {:?}, elapsed: {:?}",
                        timeout,
                        start.elapsed().as_secs()
                    ); // TODO add message
                    sleep(retry_interval);
                }
            }
        }
        drop(clt);
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("Failed to connect to "),
        ))

        // let start = Instant::now();
        // assert!(
        //     timeout > retry_interval,
        //     "timeout must be greater than retry_interval"
        // );
        // while start.elapsed() < timeout {
        //     let res = TcpStream::connect(addr);
        //     match res {
        //         Ok(stream) => {
        //             info!("connected, stream: {:?}", stream);
        //             stream.set_nodelay(true)?;
        //             // let handle = thread::spawn(move || {
        //             //     let mut buf = [0; 1024];
        //             //     while true {
        //             //         let n = stream.read(&mut buf).unwrap();
        //             //         info!("read {} bytes, buf: {:?}", n, &buf[..n]);
        //             //         callback.on_read(&buf[..n]);
        //             //     }
        //             // });
        //             return Ok(Clt { stream, callback });
        //         }
        //         Err(e) => {
        //             info!(
        //                 "not connected, due to error: {:?}, will try again in {}s",
        //                 e,
        //                 retry_interval.as_secs()
        //             );
        //             sleep(retry_interval);
        //             continue;
        //         }
        //     }
        // }
        // Err(std::io::Error::new(
        //     std::io::ErrorKind::TimedOut,
        //     format!("timeout after {}s", timeout.as_secs()),
        // ))
    }
    fn spawn_connection(&self, addr: String, retry_interval: Duration) {}
}

#[cfg(test)]
mod test {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        sync::Arc,
        time::Duration,
    };

    use log::info;

    use crate::{tcp::callback::LoggerCallback, unittest::setup};

    use super::Clt;

    #[test]
    fn test_clt() {
        setup::log::configure();
        let callback = Arc::new(LoggerCallback);
        let mut clt = Clt::new_with_timeout(
            "tcpbin.com:4242",
            // "localhost:1234",
            callback,
            Duration::from_secs(5),
            Duration::from_secs(1),
            true,
        )
        .unwrap();
        info!("clt: {:?}", clt);

        // spawn(|| {
        //     let read = &mut [0; 1024];
        //     &clt.stream.write(b"hi from the sky".as_slice());
        //     let len = clt.stream.read(read);
        //     info!("read: {:?}, read: {:?}", len, read);
        // });
    }

    #[test]
    fn test_steram() {
        setup::log::configure();
        // let mut stream = TcpStream::connect("tcpbin.com:4242").unwrap();
        let mut stream = TcpStream::connect("127.0.0.1:5000").unwrap();
        stream.set_nodelay(true).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        info!("stream: {:?}", stream);
        let size = stream.write(b"hello there").unwrap();
        info!("size: {:?}", size);
        stream.flush().unwrap();
        info!("flushed");
        let mut buf = [0; 100];
        let size = stream.read(&mut buf).unwrap();
        info!("read size: {:?}, buf: {:?}", size, &buf[..size])
    }
}
