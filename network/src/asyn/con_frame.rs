use std::{error::Error, fmt::Debug, marker::PhantomData, any::type_name};

use bytes::{Bytes, BytesMut};
use framing::FrameHandler;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

pub struct ConnectionFramed<HANDLER: FrameHandler> {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,

    phantom: PhantomData<HANDLER>, // this allows T declaration
}
impl<HANDLER: FrameHandler> Debug for ConnectionFramed<HANDLER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = type_name::<HANDLER>().split("::").last().unwrap_or("FrameHandler");
        let name = format!("ConnectionFrame<{}>", ty);
        f.debug_struct(name.as_str())
            .field("stream", &self.stream)
            .field("buffer", &self.buffer)
            .finish()
    }
}

impl<HANDLER> ConnectionFramed<HANDLER>
where
    HANDLER: FrameHandler + std::fmt::Debug,
{
    pub fn with_capacity(socket: TcpStream, capacity: usize) -> ConnectionFramed<HANDLER> {
        ConnectionFramed {
            stream: BufWriter::new(socket), // TODO figure out if the write makes performance worse
            buffer: BytesMut::with_capacity(capacity),
            phantom: PhantomData,
        }
    }
    pub fn new(socket: TcpStream) -> ConnectionFramed<HANDLER> {
        ConnectionFramed {
            stream: BufWriter::new(socket),
            buffer: BytesMut::new(),
            phantom: PhantomData,
        }
    }

    // TODO remove box in result
    pub async fn read_frame(&mut self) -> Result<Option<Bytes>, Box<dyn Error + Send + Sync>> {
        loop {
            if let Some(bytes) = HANDLER::get_frame(&mut self.buffer) {
                return Ok(Some(bytes));
            } else {
                if 0 == self.stream.read_buf(&mut self.buffer).await? {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    } else {
                        return Err("connection reset by peer".into()); // TODO add remainder of buffer to message
                    }
                }
            }
        }
    }
    pub async fn write_frame(&mut self, bytes: &[u8]) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.stream.write_all(&bytes).await?;
        self.stream.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::unittest::setup;
    use byteserde::prelude::*;
    use log::info;
    use soupbintcp4::prelude::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        let addr = setup::net::svc_default_addr();
        type SoupBinX = SoupBin<NoPayload>;
        let svc = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let listener = TcpListener::bind(addr).await.unwrap();
                let (socket, _) = listener.accept().await.unwrap();
                let mut con = ConnectionFramed::<SoupBinFrame>::with_capacity(socket, 128);
                info!("svc con: {:?}", con);
                loop {
                    let frame = con.read_frame().await.unwrap();
                    if let Some(frm) = frame {
                        let msg: SoupBinX = from_slice(&frm[..]).unwrap();
                        info!("svc: msg: {:?}", msg);
                    } else {
                        info!("svc: msg: None - Client closed connection");
                        break;
                    }
                }
            })
        };
        let clt = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let socket = TcpStream::connect(addr).await.unwrap();
                let mut con = ConnectionFramed::<SoupBinFrame>::new(socket);
                info!("clt conn: {:?}", con);
                let msg = SoupBinX::dbg(b"hello world!");
                let (slice, size): ([u8; 128], _) = to_bytes_stack(&msg).unwrap();
                let slice = &slice[..size];
                info!("clt: msg: {:?}", msg);
                con.write_frame(&slice).await.unwrap();
            })
        };
        clt.await.unwrap();
        svc.await.unwrap();
    }
}
