use std::{
    error::Error,
    fmt::{Debug, Display},
    marker::PhantomData,
};

use bytes::{Bytes, BytesMut};
use framing::Framer;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

#[derive(Debug)]
pub struct FrameReader<HANDLER: Framer> {
    reader: OwnedReadHalf,
    buffer: BytesMut,
    phantom: PhantomData<HANDLER>,
}
impl<HANDLER: Framer> Display for FrameReader<HANDLER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {:?}->{:?} }}",
            std::any::type_name::<HANDLER>(),
            self.reader
                .local_addr()
                .expect("could not get reader's local address"),
            self.reader
                .peer_addr()
                .expect("could not get reader's peer address"),
        )
    }
}
impl<HANDLER: Framer> FrameReader<HANDLER> {
    pub fn with_capacity(reader: OwnedReadHalf, capacity: usize) -> FrameReader<HANDLER> {
        Self {
            reader,
            buffer: BytesMut::with_capacity(capacity),
            phantom: PhantomData,
        }
    }

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
pub struct FrameWriter {
    writer: OwnedWriteHalf,
}
impl Display for FrameWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameWriter {{ {:?}->{:?} }}",
            self.writer
                .local_addr()
                .expect("could not get reader's local address"),
            self.writer
                .peer_addr()
                .expect("could not get reader's peer address"),
        )
    }
}
impl FrameWriter {
    pub fn new(writer: OwnedWriteHalf) -> FrameWriter {
        Self { writer }
    }
    pub async fn write_frame(&mut self, bytes: &[u8]) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.writer.write_all(bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
}

type FrameManger<HANDLER> = (FrameReader<HANDLER>, FrameWriter);
fn into_split_frame_manager<HANDLER: Framer>(
    stream: TcpStream,
    reader_capacity: usize,
) -> FrameManger<HANDLER> {
    match stream.into_split() {
        (r, w) => (
            FrameReader::<HANDLER>::with_capacity(r, reader_capacity),
            FrameWriter::new(w),
        ),
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::unittest::setup;
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use log::info;
    use soupbintcp4::prelude::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        const CAP: usize = 1024;
        let addr = setup::net::default_addr();
        type SoupBinX = SoupBinMsg<NoPayload>;
        let svc = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let listener = TcpListener::bind(addr).await.unwrap();
                let (stream, _) = listener.accept().await.unwrap();

                let (mut reader, mut writer) = into_split_frame_manager::<SoupBinFramer>(stream, CAP);
                info!("svc: writer: {}, reader: {}", writer, reader);
                loop {
                    let frame = reader.read_frame().await.unwrap();
                    if let Some(frm) = frame {
                        info!("svc: read_frame: \n{}", to_hex_pretty(&frm[..]));
                        let msg: SoupBinX = from_slice(&frm[..]).unwrap();
                        info!("svc: from_slice: {:?}", msg);

                        let msg = SoupBinX::dbg(b"Hello From Server");
                        let (slice, size): ([u8; CAP], _) = to_bytes_stack(&msg).unwrap();
                        writer.write_frame(&slice[..size]).await.unwrap();
                        info!("svc: write_frame: \n{}", to_hex_pretty(&slice[..size]))
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
                let stream = TcpStream::connect(addr).await.unwrap();
                let (mut reader, mut writer) = into_split_frame_manager::<SoupBinFramer>(stream, CAP);
                info!("clt: writer: {}, reader: {}", writer, reader);
                let msg = SoupBinX::dbg(b"Hello From Client");
                let (slice, size): ([u8; CAP], _) = to_bytes_stack(&msg).unwrap();
                let slice = &slice[..size];
                writer.write_frame(slice).await.unwrap();
                info!("clt: write_frame: \n{}", to_hex_pretty(slice));
                let frame = reader.read_frame().await.unwrap().unwrap();

                info!("clt: read_frame: \n{}", to_hex_pretty(&frame[..]));
                let msg: SoupBinX = from_slice(&frame[..]).unwrap();
                info!("clt: from_slice: {:?}", msg);
            })
        };
        clt.await.unwrap();
        svc.await.unwrap();
    }
}
