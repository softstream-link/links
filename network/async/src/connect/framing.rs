use std::{
    error::Error,
    fmt::{Debug, Display},
    marker::PhantomData,
};

use bytes::{Bytes, BytesMut};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

use links_network_core::prelude::Framer;

#[derive(Debug)]
pub struct FrameReader<F: Framer> {
    reader: OwnedReadHalf,
    buffer: BytesMut,
    phantom: PhantomData<F>,
}
impl<F: Framer> FrameReader<F> {
    pub fn with_capacity(reader: OwnedReadHalf, capacity: usize) -> FrameReader<F> {
        Self {
            reader,
            buffer: BytesMut::with_capacity(F::MAX_FRAME_SIZE),
            phantom: PhantomData,
        }
    }
    pub async fn read_frame(&mut self) -> Result<Option<Bytes>, Box<dyn Error+Send+Sync>> {
        loop {
            if let Some(bytes) = F::get_frame(&mut self.buffer) {
                return Ok(Some(bytes));
            } else {
                match self.reader.read_buf(&mut self.buffer).await? {
                    0 => {
                        if self.buffer.is_empty() {
                            return Ok(None);
                        } else {
                            return Err("connection reset by peer".into()); // TODO add remainder of buffer to message
                        }
                    }
                    _ => {
                        continue;
                    }
                }
            }
        }
    }
}
impl<F: Framer> Display for FrameReader<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {:?}->{:?} }}",
            std::any::type_name::<F>()
                .split("::")
                .last()
                .unwrap_or("Unknown"),
            self.reader
                .local_addr()
                .expect("could not get reader's local address"),
            self.reader
                .peer_addr()
                .expect("could not get reader's peer address"),
        )
    }
}

#[derive(Debug)]
pub struct FrameWriter {
    writer: OwnedWriteHalf,
}
impl FrameWriter {
    pub fn new(writer: OwnedWriteHalf) -> FrameWriter {
        Self { writer }
    }
    pub async fn write_frame(&mut self, bytes: &[u8]) -> Result<(), Box<dyn Error+Send+Sync>> {
        self.writer.write_all(bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
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

type FrameManger<F> = (FrameReader<F>, FrameWriter);

pub fn into_split_frame_manager<F: Framer>(
    stream: TcpStream,
    reader_capacity: usize,
) -> FrameManger<F> {
    let (reader, writer) = stream.into_split();
    (
        FrameReader::<F>::with_capacity(reader, reader_capacity),
        FrameWriter::new(writer),
    )
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::unittest::setup::protocol::*;
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use links_testing::unittest::setup;
    use links_testing::unittest::setup::model::*;
    use log::info;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        const CAP: usize = 128;
        let inp_svc_msg = TestSvcMsgDebug::new(b"Hello Server Frame");
        let addr = setup::net::rand_avail_addr_port();
        let svc = {
            tokio::spawn({
                let inp_svc_msg = inp_svc_msg.clone();
                async move {
                    let listener = TcpListener::bind(addr).await.unwrap();
                    let (stream, _) = listener.accept().await.unwrap();

                    let (mut reader, mut writer) =
                        into_split_frame_manager::<TestSvcMsgProtocol>(stream, CAP);
                    info!("svc: writer: {}, reader: {}", writer, reader);
                    let mut out_svc_msg: Option<TestCltMsgDebug> = None;
                    loop {
                        let frame = reader.read_frame().await.unwrap();
                        if let Some(frm) = frame {
                            let msg: TestCltMsgDebug = from_slice(&frm[..]).unwrap();
                            out_svc_msg = Some(msg);

                            let (slice, size): ([u8; CAP], _) =
                                to_bytes_stack(&inp_svc_msg).unwrap();
                            let slice = &slice[..size];
                            writer.write_frame(slice).await.unwrap();
                            info!("svc: write_frame: \n{}", to_hex_pretty(slice))
                        } else {
                            info!("svc: msg: None - Client closed connection");
                            break;
                        }
                    }
                    out_svc_msg.unwrap()
                }
            })
        };
        let inp_clt_msg = TestCltMsgDebug::new(b"Hello Client Frame");
        let clt = {
            tokio::spawn({
                let inp_clt_msg = inp_clt_msg.clone();
                async move {
                    let stream = TcpStream::connect(addr).await.unwrap();
                    let (mut reader, mut writer) =
                        into_split_frame_manager::<TestCltMsgProtocol>(stream, CAP);

                    info!("clt: writer: {}, reader: {}", writer, reader);
                    let (slice, size): ([u8; CAP], _) = to_bytes_stack(&inp_clt_msg).unwrap();
                    let slice = &slice[..size];
                    info!("clt: write_frame: \n{}", to_hex_pretty(slice));
                    writer.write_frame(slice).await.unwrap();

                    let frame = reader.read_frame().await.unwrap().unwrap();
                    let out_clt_msg: TestSvcMsgDebug = from_slice(&frame[..]).unwrap();
                    out_clt_msg
                }
            })
        };
        let out_clt_msg = clt.await.unwrap();
        let out_svc_msg = svc.await.unwrap();
        info!("inp_clt_msg: {:?}", inp_clt_msg);
        info!("out_clt_msg: {:?}", out_clt_msg);
        info!("inp_svc_msg: {:?}", inp_svc_msg);
        info!("out_svc_msg: {:?}", out_svc_msg);
        assert_eq!(inp_clt_msg, out_svc_msg);
        assert_eq!(inp_svc_msg, out_clt_msg);
    }
}
