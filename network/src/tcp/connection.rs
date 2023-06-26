use std::io::prelude::*;
use std::{
    io::{ErrorKind, Write},
    net::{Shutdown, TcpStream},
    thread::sleep,
    time::{Duration, Instant},
};

use log::{info, warn};

struct Connection {
    addr: String,
    stream: TcpStream,
    connect_timeout: Duration,
    retry_interval: Duration,
}

impl Connection {
    pub fn new(
        addr: &str,
        connect_timeout: Duration,
        retry_interval: Duration,
    ) -> Result<Self, std::io::Error> {
        assert!(connect_timeout > retry_interval);
        assert!(retry_interval > Duration::from_secs(0));

        Ok(Connection {
            addr: addr.to_owned(),
            stream: Self::try_connect(addr, connect_timeout, retry_interval)?,
            connect_timeout,
            retry_interval,
        })
    }

    fn try_connect(
        addr: &str,
        connect_timeout: Duration,
        retry_interval: Duration,
    ) -> Result<TcpStream, std::io::Error> {
        let start = Instant::now();
        let mut last_error;
        loop {
            let res = TcpStream::connect(addr);
            match res {
                Ok(stream) => {
                    return Ok(stream);
                }
                Err(e) => {
                    last_error = e;
                    info!(
                        "{:?} elapsed: {:?}s, retry_interval: {:?}s, connect_timeout: {:?}s",
                        last_error,
                        start.elapsed().as_secs(),
                        retry_interval.as_secs(),
                        connect_timeout.as_secs(),
                    );
                }
            }
            if start.elapsed() > connect_timeout {
                break;
            }
            sleep(retry_interval);
        }

        Err(std::io::Error::new(
            ErrorKind::TimedOut,
            format!(
                "{:?}, timeout after {:?}s",
                last_error,
                connect_timeout.as_secs()
            ),
        ))
    }
    fn reconnect(&mut self, reason: &str) -> Result<(), std::io::Error> {
        info!(
            "Shutdown START stream: {:?}, reason: {}",
            self.stream, reason
        );
        let res = self.stream.shutdown(Shutdown::Both);
        match res {
            Ok(_) => info!("Shutdown DONE Ok({:?}): ", self.stream),
            Err(e) => warn!("Shutdown DONE Err({:?})", e),
        }
        info!("Re-Connect START addr: {:?}", self.addr);
        let res = Self::try_connect(&self.addr, self.connect_timeout, self.retry_interval);
        match res {
            Ok(stream) => {
                info!("Re-Connect DONE Ok({:?}): ", stream);
                self.stream = stream;
                Ok(())
            }
            Err(e) => {
                warn!("Re-Connect DONE Err({:?})", e);
                Err(e)
            }
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.stream.write_all(data)?;
        Ok(())
    }
    pub fn read(&mut self, data: &mut [u8]) -> Result<usize, std::io::Error> {
        let size = self.stream.read(data)?;
        if size > 0 {
            return Ok(size);
        }
        
        const MAX: i32 = 2;
        for attempt in (1..=MAX) {
            let msg = format!(
                "read attempt: {}, return size: {}, indicates server closed connection",
                attempt, size
            );
            let _ = self.reconnect(msg.as_str());
            let size = self.stream.read(data)?;
            if size > 0 {
                return Ok(size);
            }
        }

        Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            format!("Read returns 0 bytes after attempt {}", MAX),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::unittest::setup;

    use super::*;

    #[test]
    fn test_connection() {
        setup::log::configure();
        let addr = "localhost:5000";
        let connect_timeout = Duration::from_secs(5);
        let retry_interval = Duration::from_secs(1);
        let mut con = Connection::new(addr, connect_timeout, retry_interval).unwrap();
        let mut data = [0; 1024];
        let x = con.read(&mut data).unwrap();
        info!("read {} bytes, data: {:?}", x, &data[..x]);
        let data = b"hello world";
        con.send(data).unwrap();
    }
}
