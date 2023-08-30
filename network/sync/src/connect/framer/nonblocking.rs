use std::{error::Error, fmt::Display, net::TcpStream};

use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use links_network_core::prelude::Framer;
use mio::net::TcpStream as TcpStreamMio;
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

pub struct FrameReader<F: Framer> {
    reader: TcpStreamMio,
    buffer: BytesMut,
    max_frame_size: usize,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer> FrameReader<F> {
    pub fn with_max_frame_size(reader: TcpStreamMio, max_frame_size: usize) -> FrameReader<F> {
        Self {
            reader,
            buffer: BytesMut::with_capacity(max_frame_size),
            max_frame_size,
            phantom: std::marker::PhantomData,
        }
    }
    #[inline]
    pub fn read_frame(&mut self) -> Result<ReadStatus<Bytes>, Box<dyn Error>> {
        loop {
            if let Some(bytes) = F::get_frame(&mut self.buffer) {
                return Ok(ReadStatus::Completed(Some(bytes)));
            } else {
                if self.buffer.capacity() < self.max_frame_size {
                    self.buffer.reserve(self.max_frame_size);
                }
                // in non blocking mode this debug-assert can fail on linux unless the reserver is done conditionally
                // this assert is here to ensure allocation is never required since after each frame is written here it is 
                // immediately read and converted into a message which means that the buffer space can be reclaimed before
                // next read from socket and therefore no allocation is required.
                debug_assert_eq!(self.buffer.capacity(), self.max_frame_size);

                let residual = self.buffer.len();
                unsafe {
                    self.buffer.set_len(self.buffer.capacity());
                }
                match self.reader.read(&mut self.buffer[residual..]) {
                    Ok(EOF) => {
                        if residual == 0 {
                            return Ok(ReadStatus::Completed(None));
                        } else {
                            unsafe { self.buffer.set_len(residual) };
                            return Err(format!(
                                "connection reset by peer residual buf: \n{}",
                                to_hex_pretty(&self.buffer[..])
                            )
                            .into());
                        }
                    }
                    Ok(len) => unsafe {
                        self.buffer.set_len(residual + len);
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        unsafe {
                            self.buffer.set_len(residual);
                        }
                        return Ok(ReadStatus::NotReady);
                    }
                    Err(e) => {
                        return Err(format!("read error: {}", e.to_string()).into());
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
        match self.writer.write_all(&bytes) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(WriteStatus::NotReady);
            }
            Err(e) => {
                return Err(format!("write error: {}", e.to_string()).into());
            }
            Ok(_) => {}
        }

        self.writer.flush()?;
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

type FrameProcessor<F> = (FrameReader<F>, FrameWriter);
pub fn into_split_framer<F: Framer>(
    stream: TcpStream,
    reader_max_frame_size: usize,
) -> FrameProcessor<F> {
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
        FrameReader::<F>::with_max_frame_size(reader, reader_max_frame_size),
        FrameWriter::new(writer),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        net::TcpListener,
        thread::{self, sleep},
        time::Duration,
    };

    use byteserde::utils::hex::to_hex_pretty;
    use links_testing::unittest::setup;
    use log::{error, info};
    use num_format::{Locale, ToFormattedString};

    #[test]
    fn test_reader() {
        setup::log::configure_level(log::LevelFilter::Info);
        const TEST_SEND_FRAME_SIZE: usize = 128;
        const WRITE_N_TIMES: usize = 100_000;
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

        let random_frame = setup::data::random_bytes(TEST_SEND_FRAME_SIZE);
        info!("sending_frame: \n{}", to_hex_pretty(random_frame));

        let addr = setup::net::rand_avail_addr_port();

        // CONFIGURE svc
        let svc = thread::Builder::new()
            .name("Thread-Svc".to_owned())
            .spawn({
                move || {
                    let listener = TcpListener::bind(addr).unwrap();
                    let (stream, _) = listener.accept().unwrap();
                    let (mut reader, _) =
                        into_split_framer::<MsgFramer>(stream, TEST_SEND_FRAME_SIZE);
                    info!("svc: reader: {}", reader);
                    let mut frame_recv_count = 0_usize;
                    loop {
                        let res = reader.read_frame();
                        match res {
                            Ok(ReadStatus::Completed(None)) => {
                                info!("svc: read_frame is None, client closed connection");
                                break;
                            }
                            Ok(ReadStatus::Completed(Some(_))) => {
                                frame_recv_count += 1;
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
            into_split_framer::<MsgFramer>(TcpStream::connect(addr).unwrap(), TEST_SEND_FRAME_SIZE);

        info!("clt: {}", writer);

        let mut frame_send_count = 0_usize;
        for _ in 0..WRITE_N_TIMES {
            loop {
                match writer.write_frame(random_frame) {
                    Ok(WriteStatus::Completed) => {
                        frame_send_count += 1;
                        break;
                    }
                    Ok(WriteStatus::NotReady) => {
                        continue;
                    }
                    Err(e) => {
                        error!("clt write_frame error: {}", e.to_string());
                        break;
                    }
                }
            }
        }
        info!(
            "frame_send_count: {}",
            frame_send_count.to_formatted_string(&Locale::en)
        );

        drop(writer);
        let frame_recv_count = svc.join().unwrap();
        info!(
            "frame_recv_count: {}",
            frame_recv_count.to_formatted_string(&Locale::en)
        );
        assert_eq!(frame_send_count, frame_recv_count);
    }
}
