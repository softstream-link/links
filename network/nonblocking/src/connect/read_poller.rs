use std::error::Error;

use mio::{net::TcpStream, Poll};

#[derive(Debug)]
pub struct Poller {
    pool: Poll,
}

impl Poller {
    pub fn new() -> Self {
        Self {
            pool: Poll::new().unwrap(),
        }
    }
    pub fn register_read(&mut self, stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
        // self.pool.register (fd, token, interest, opts)
        Ok(())
    }
}
