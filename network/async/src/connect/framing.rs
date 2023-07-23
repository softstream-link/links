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

use crate::core::Framer;

#[derive(Debug)]
pub struct FrameReader<FRAMER: Framer> {
    reader: OwnedReadHalf,
    buffer: BytesMut,
    phantom: PhantomData<FRAMER>,
}
impl<FRAMER: Framer> Display for FrameReader<FRAMER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {:?}->{:?} }}",
            std::any::type_name::<FRAMER>(),
            self.reader
                .local_addr()
                .expect("could not get reader's local address"),
            self.reader
                .peer_addr()
                .expect("could not get reader's peer address"),
        )
    }
}
impl<FRAMER: Framer> FrameReader<FRAMER> {
    pub fn with_capacity(reader: OwnedReadHalf, capacity: usize) -> FrameReader<FRAMER> {
        Self {
            reader,
            buffer: BytesMut::with_capacity(capacity),
            phantom: PhantomData,
        }
    }
    // TODO error types for this crait
    pub async fn read_frame(&mut self) -> Result<Option<Bytes>, Box<dyn Error + Send + Sync>> {
        loop {
            if let Some(bytes) = FRAMER::get_frame(&mut self.buffer) {
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

type FrameManger<FRAMER> = (FrameReader<FRAMER>, FrameWriter);

pub fn into_split_frame_manager<FRAMER: Framer>(
    stream: TcpStream,
    reader_capacity: usize,
) -> FrameManger<FRAMER> {
    match stream.into_split() {
        (r, w) => (
            FrameReader::<FRAMER>::with_capacity(r, reader_capacity),
            FrameWriter::new(w),
        ),
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::unittest::setup;
    use crate::unittest::setup::model::*;
    use crate::unittest::setup::protocol::*;
    use byteserde::{prelude::*, utils::hex::to_hex_pretty};
    use log::info;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        const CAP: usize = 128;
        let addr = setup::net::default_addr();
        let inp_svc_msg = Msg::Svc(MsgFromSvc::new(b"Hello Server Frame"));

        let svc = {
            let addr = addr.clone();
            tokio::spawn({
                let inp_svc_msg = inp_svc_msg.clone();
                async move {
                    let listener = TcpListener::bind(addr).await.unwrap();
                    let (stream, _) = listener.accept().await.unwrap();

                    let (mut reader, mut writer) =
                        into_split_frame_manager::<MsgProtocolHandler>(stream, CAP);
                    info!("svc: writer: {}, reader: {}", writer, reader);
                    let mut out_svc_msg: Option<Msg> = None;
                    loop {
                        let frame = reader.read_frame().await.unwrap();
                        if let Some(frm) = frame {
                            let msg: Msg = from_slice(&frm[..]).unwrap();
                            out_svc_msg = Some(msg);

                            let (slice, size): ([u8; CAP], _) =
                                to_bytes_stack(&inp_svc_msg).unwrap();
                            writer.write_frame(&slice[..size]).await.unwrap();
                            info!("svc: write_frame: \n{}", to_hex_pretty(&slice[..size]))
                        } else {
                            info!("svc: msg: None - Client closed connection");
                            break;
                        }
                    }
                    out_svc_msg.unwrap()
                }
            })
        };
        let inp_clt_msg = Msg::Clt(MsgFromClt::new(b"Hello Client Frame"));
        let clt = {
            let addr = addr.clone();
            tokio::spawn({
                let inp_clt_msg = inp_clt_msg.clone();
                async move {
                    let stream = TcpStream::connect(addr).await.unwrap();
                    let (mut reader, mut writer) =
                        into_split_frame_manager::<MsgProtocolHandler>(stream, CAP);

                    info!("clt: writer: {}, reader: {}", writer, reader);
                    let (slice, size): ([u8; CAP], _) = to_bytes_stack(&inp_clt_msg).unwrap();
                    let slice = &slice[..size];
                    info!("clt: write_frame: \n{}", to_hex_pretty(slice));
                    writer.write_frame(slice).await.unwrap();

                    let frame = reader.read_frame().await.unwrap().unwrap();
                    let out_clt_msg: Msg = from_slice(&frame[..]).unwrap();
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
    // TODO move to soupbintcp4 and create from local msg
    // #[tokio::test]
    // async fn test_connection() {
    //     setup::log::configure();
    //     const CAP: usize = 1024;
    //     let addr = setup::net::default_addr();
    //     type SoupBinX = SoupBinMsg<NoPayload>;
    //     let svc = {
    //         let addr = addr.clone();
    //         tokio::spawn(async move {
    //             let listener = TcpListener::bind(addr).await.unwrap();
    //             let (stream, _) = listener.accept().await.unwrap();

    //             let (mut reader, mut writer) =
    //                 into_split_frame_manager::<SoupBinFramer>(stream, CAP);
    //             info!("svc: writer: {}, reader: {}", writer, reader);
    //             loop {
    //                 let frame = reader.read_frame().await.unwrap();
    //                 if let Some(frm) = frame {
    //                     info!("svc: read_frame: \n{}", to_hex_pretty(&frm[..]));
    //                     let msg: SoupBinX = from_slice(&frm[..]).unwrap();
    //                     info!("svc: from_slice: {:?}", msg);

    //                     let msg = SoupBinX::dbg(b"Hello From Server");
    //                     let (slice, size): ([u8; CAP], _) = to_bytes_stack(&msg).unwrap();
    //                     writer.write_frame(&slice[..size]).await.unwrap();
    //                     info!("svc: write_frame: \n{}", to_hex_pretty(&slice[..size]))
    //                 } else {
    //                     info!("svc: msg: None - Client closed connection");
    //                     break;
    //                 }
    //             }
    //         })
    //     };
    //     let clt = {
    //         let addr = addr.clone();
    //         tokio::spawn(async move {
    //             let stream = TcpStream::connect(addr).await.unwrap();
    //             let (mut reader, mut writer) =
    //                 into_split_frame_manager::<SoupBinFramer>(stream, CAP);
    //             info!("clt: writer: {}, reader: {}", writer, reader);
    //             let msg = SoupBinX::dbg(b"Hello From Client");
    //             let (slice, size): ([u8; CAP], _) = to_bytes_stack(&msg).unwrap();
    //             let slice = &slice[..size];
    //             writer.write_frame(slice).await.unwrap();
    //             info!("clt: write_frame: \n{}", to_hex_pretty(slice));
    //             let frame = reader.read_frame().await.unwrap().unwrap();

    //             info!("clt: read_frame: \n{}", to_hex_pretty(&frame[..]));
    //             let msg: SoupBinX = from_slice(&frame[..]).unwrap();
    //             info!("clt: from_slice: {:?}", msg);
    //         })
    //     };
    //     clt.await.unwrap();
    //     svc.await.unwrap();
    // }
}
