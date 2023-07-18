use std::{any::type_name, error::Error, fmt::Debug, marker::PhantomData};

use bytes::{Bytes, BytesMut};
use framing::Framer;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}, TcpStream},
};

#[derive(Debug)]
pub struct StreamReadFramer<HANDLER: Framer> {
    reader: OwnedReadHalf,
    buffer: BytesMut,
    phantom: PhantomData<HANDLER>, // this allows T declaration
}
impl<HANDLER: Framer> StreamReadFramer<HANDLER> {
    pub fn new(reader: OwnedReadHalf) -> StreamReadFramer<HANDLER> {
        Self {
            reader: reader,
            buffer: BytesMut::new(),
            phantom: PhantomData,
        }
    }
    pub fn with_capacity(reader: OwnedReadHalf, capacity: usize) -> StreamReadFramer<HANDLER> {
        Self {
            reader: reader,
            buffer: BytesMut::with_capacity(capacity),
            phantom: PhantomData,
        }
    }
    // TODO remove box in result
    pub async fn read_frame(&mut self) -> Result<Option<Bytes>, Box<dyn Error + Send + Sync>> {
        loop {
            if let Some(bytes) = HANDLER::get_frame(&mut self.buffer) {
                return Ok(Some(bytes));
            } else {
                if 0 == self.reader.read_buf(&mut self.buffer).await? {
                    if self.buffer.is_empty() {
                        return Ok(None);
                    } else {
                        return Err("connection reset by peer".into()); // TODO add remainder of buffer to message
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct StreamWriteFramer<HANDLER: Framer> {
    writer: BufWriter<OwnedWriteHalf>,
    phantom: PhantomData<HANDLER>, // this allows T declaration
}

impl<HANDLER: Framer> StreamWriteFramer<HANDLER> {
    pub fn new(writer: OwnedWriteHalf) -> StreamWriteFramer<HANDLER> {
        Self {
            writer: BufWriter::new(writer),
            phantom: PhantomData,
        }
    }
    pub fn with_capacity(
        writer: OwnedWriteHalf,
        capacity: usize,
    ) -> StreamWriteFramer<HANDLER> {
        Self {
            writer: BufWriter::with_capacity(capacity, writer),
            phantom: PhantomData,
        }
    }
    pub async fn write_frame(&mut self, bytes: &[u8]) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.writer.write_all(bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

pub struct StreamFramer<HANDLER: Framer> {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,

    phantom: PhantomData<HANDLER>, // this allows T declaration
}
impl<HANDLER: Framer> Debug for StreamFramer<HANDLER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = type_name::<HANDLER>()
            .split("::")
            .last()
            .unwrap_or("FrameHandler");
        let name = format!("ConnectionFrame<{}>", ty);
        f.debug_struct(name.as_str())
            .field("stream", &self.stream)
            .field("buffer", &self.buffer)
            .finish()
    }
}

impl<HANDLER> StreamFramer<HANDLER>
where
    HANDLER: Framer + std::fmt::Debug,
{
    pub fn with_capacity(stream: TcpStream, capacity: usize) -> StreamFramer<HANDLER> {
        StreamFramer {
            stream: BufWriter::new(stream), // TODO figure out if the write makes performance worse
            buffer: BytesMut::with_capacity(capacity),
            phantom: PhantomData,
        }
    }
    pub fn new(stream: TcpStream) -> StreamFramer<HANDLER> {
        StreamFramer {
            stream: BufWriter::new(stream),
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
        let addr = setup::net::default_addr();
        type SoupBinX = SoupBinMsg<NoPayload>;
        let svc = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let listener = TcpListener::bind(addr).await.unwrap();
                let (socket, _) = listener.accept().await.unwrap();
                let mut con = StreamFramer::<SoupBinFramer>::with_capacity(socket, 128);
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
                let mut con = StreamFramer::<SoupBinFramer>::new(socket);
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
