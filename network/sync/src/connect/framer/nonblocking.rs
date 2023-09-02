use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use links_network_core::prelude::Framer;
use mio::net::TcpStream as TcpStreamMio;
use std::mem::MaybeUninit;
use std::{error::Error, fmt::Display, net::TcpStream};
// use std::net::TcpStream as TcpStreamMio;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

const EOF: usize = 0;
#[derive(Debug)]
pub enum ReadStatus<T> {
    Completed(Option<T>),
    NotReady,
}
pub enum WriteStatus {
    Completed,
    NotReady,
}

// TODO evaluate if it is possible to use unsafe set_len on buf then we would not need a MAX_MESSAGE_SIZE generic as it can just be an non const arg to new
pub struct FrameReader<F: Framer, const MAX_MESSAGE_SIZE: usize> {
    reader: TcpStreamMio,
    buffer: BytesMut,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer, const MAX_MESSAGE_SIZE: usize> FrameReader<F, MAX_MESSAGE_SIZE> {
    pub fn new(reader: TcpStreamMio) -> FrameReader<F, MAX_MESSAGE_SIZE> {
        Self {
            reader,
            buffer: BytesMut::with_capacity(MAX_MESSAGE_SIZE),
            phantom: std::marker::PhantomData,
        }
    }
    #[inline]
    pub fn read_frame(&mut self) -> Result<ReadStatus<Bytes>, Box<dyn Error>> {
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [u8; MAX_MESSAGE_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.reader.read(&mut buf) {
            Ok(EOF) => {
                if self.buffer.is_empty() {
                    Ok(ReadStatus::Completed(None))
                } else {
                    let msg = format!(
                        "connection reset by peer, residual buf:\n{}",
                        to_hex_pretty(&self.buffer[..])
                    );
                    Err(msg.into())
                }
            }
            Ok(len) => {
                self.buffer.extend_from_slice(&buf[..len]);
                if let Some(bytes) = F::get_frame(&mut self.buffer) {
                    Ok(ReadStatus::Completed(Some(bytes)))
                } else {
                    Ok(ReadStatus::NotReady)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(ReadStatus::NotReady),
            Err(e) => Err(format!("read error: {}", e).into()),
        }
    }
}
impl<F: Framer, const MAX_MESSAGE_SIZE: usize> Display for FrameReader<F, MAX_MESSAGE_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {:?}->{:?}, fd: {} }}",
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
            self.reader.as_raw_fd(),
        )
    }
}

pub struct FrameWriter {
    writer: TcpStreamMio,
}
impl FrameWriter {
    pub fn new(stream: TcpStreamMio) -> Self {
        Self { writer: stream }
    }
    #[inline]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<WriteStatus, Box<dyn Error>> {
        let mut residual = bytes;
        while !residual.is_empty() {
            match self.writer.write(residual) {
                // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
                #[rustfmt::skip]
                Ok(0) => {
                    let msg = format!("connection reset by peer, residual buf:\n{}", to_hex_pretty(residual));
                    return Err(msg.into());
                }
                Ok(len) => {
                    if len == residual.len() {
                        return Ok(WriteStatus::Completed);
                    } else {
                        residual = &residual[len..];
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // repeteat untill frame is written entirely in busy loop
                    continue;
                }
                Err(e) => {
                    let msg = format!("write error: {}, residual:\n{}", e, to_hex_pretty(residual));
                    return Err(msg.into());
                }
            }
        }
        Ok(WriteStatus::Completed)
    }
}
impl Display for FrameWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameWriter {{ {:?}->{:?}, fd: {} }}",
            self.writer
                .local_addr()
                .expect("could not get reader's local address"),
            self.writer
                .peer_addr()
                .expect("could not get reader's peer address"),
            self.writer.as_raw_fd(),
        )
    }
}

type FrameProcessor<F, const MAX_MESSAGE_SIZE: usize> =
    (FrameReader<F, MAX_MESSAGE_SIZE>, FrameWriter);
pub fn into_split_framer<F: Framer, const MAX_MESSAGE_SIZE: usize>(
    stream: TcpStream,
) -> FrameProcessor<F, MAX_MESSAGE_SIZE> {
    stream
        .set_nonblocking(true)
        .expect("Failed to set_nonblocking on TcpStream");
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for FrameReader"),
        stream,
    );
    let (reader, writer) = (
        TcpStreamMio::from_std(reader),
        TcpStreamMio::from_std(writer),
    );
    (
        FrameReader::<F, MAX_MESSAGE_SIZE>::new(reader),
        FrameWriter::new(writer),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        net::TcpListener,
        thread::{self, sleep},
        time::{Duration, Instant},
    };

    use byteserde::utils::hex::to_hex_pretty;
    use links_testing::unittest::setup;
    use log::{error, info};
    use num_format::{Locale, ToFormattedString};

    #[test]
    fn test_reader() {
        setup::log::configure_level(log::LevelFilter::Info);
        const TEST_SEND_FRAME_SIZE: usize = 128;
        const WRITE_N_TIMES: usize = 10_000_000;
        pub struct MsgFramer;
        impl Framer for MsgFramer {
            fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
                if bytes.len() < TEST_SEND_FRAME_SIZE {
                    return None;
                } else {
                    let frame = bytes.split_to(TEST_SEND_FRAME_SIZE);
                    return Some(frame.freeze());
                }
            }
        }

        let send_frame = setup::data::random_bytes(TEST_SEND_FRAME_SIZE);
        info!("sending_frame: \n{}", to_hex_pretty(send_frame));

        let addr = setup::net::rand_avail_addr_port();

        // CONFIGURE svc
        let svc = thread::Builder::new()
            .name("Thread-Svc".to_owned())
            .spawn({
                move || {
                    let listener = TcpListener::bind(addr).unwrap();
                    let (stream, _) = listener.accept().unwrap();
                    let (mut reader, _) =
                        into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(stream);
                    info!("svc: reader: {}", reader);
                    let mut frame_recv_count = 0_usize;
                    loop {
                        let res = reader.read_frame();
                        match res {
                            Ok(ReadStatus::Completed(None)) => {
                                info!("svc: read_frame is None, client closed connection");
                                break;
                            }
                            Ok(ReadStatus::Completed(Some(recv_frame))) => {
                                frame_recv_count += 1;
                                let recv_frame = &recv_frame[..];
                                assert_eq!(
                                    send_frame,
                                    recv_frame,
                                    "send_frame: \n{}\nrecv_frame:\n{}\nframe_recv_count: {}",
                                    to_hex_pretty(send_frame),
                                    to_hex_pretty(recv_frame),
                                    frame_recv_count
                                );
                            }
                            Ok(ReadStatus::NotReady) => {
                                continue; // try reading again
                            }
                            Err(e) => {
                                error!("Svc read_rame error: {}", e.to_string());
                                break;
                            }
                        }
                    }
                    frame_recv_count
                }
            })
            .unwrap();

        sleep(Duration::from_millis(100)); // allow the spawned to bind
                                           // CONFIGUR clt
        let (_, mut writer) =
            into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(TcpStream::connect(addr).unwrap());

        info!("clt: {}", writer);

        let mut frame_send_count = 0_usize;
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            loop {
                match writer.write_frame(send_frame) {
                    Ok(WriteStatus::Completed) => {
                        frame_send_count += 1;
                        break;
                    }
                    Ok(WriteStatus::NotReady) => {
                        continue;
                    }
                    Err(e) => {
                        panic!("clt write_frame error: {}", e.to_string());
                    }
                }
            }
        }
        let elapsed = start.elapsed();

        drop(writer);
        let frame_recv_count = svc.join().unwrap();
        info!(
            "frame_send_count: {}, frame_recv_count: {}",
            frame_send_count.to_formatted_string(&Locale::en),
            frame_recv_count.to_formatted_string(&Locale::en)
        );
        info!(
            "per send elapsed: {:?}, total elapsed: {:?} ",
            elapsed / WRITE_N_TIMES as u32,
            elapsed
        );
        assert_eq!(frame_send_count, frame_recv_count);
        assert_eq!(frame_send_count, WRITE_N_TIMES);
    }
}
