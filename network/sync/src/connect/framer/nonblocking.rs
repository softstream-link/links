use crate::prelude_nonblocking::{ReadStatus, WriteStatus};
use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use links_network_core::prelude::Framer;
use std::{
    fmt::Display,
    io::{Error, ErrorKind, Read, Write},
};

use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;

use log::{debug, log_enabled};
const EOF: usize = 0;

// TODO evaluate if it is possible to use unsafe set_len on buf then we would not need a MAX_MSG_SIZE generic as it can just be an non const arg to new
#[derive(Debug)]
pub struct FrameReader<F: Framer, const MAX_MSG_SIZE: usize> {
    pub(crate) stream_reader: mio::net::TcpStream,
    buffer: BytesMut,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer, const MAX_MSG_SIZE: usize> FrameReader<F, MAX_MSG_SIZE> {
    pub fn new(reader: mio::net::TcpStream) -> FrameReader<F, MAX_MSG_SIZE> {
        Self {
            stream_reader: reader,
            buffer: BytesMut::with_capacity(MAX_MSG_SIZE),
            phantom: std::marker::PhantomData,
        }
    }
    #[inline]
    pub fn read_frame(&mut self) -> Result<ReadStatus<Bytes>, Error> {
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [u8; MAX_MSG_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.stream_reader.read(&mut buf) {
            Ok(EOF) => {
                if self.buffer.is_empty() {
                    Ok(ReadStatus::Completed(None))
                } else {
                    let msg = format!(
                        "connection reset by peer, residual buf:\n{}",
                        to_hex_pretty(&self.buffer[..])
                    );
                    Err(Error::new(ErrorKind::ConnectionReset, msg))
                }
            }
            Ok(len) => {
                self.buffer.extend_from_slice(&buf[..len]);
                if let Some(bytes) = F::get_frame(&mut self.buffer) {
                    Ok(ReadStatus::Completed(Some(bytes)))
                } else {
                    Ok(ReadStatus::WouldBlock)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(ReadStatus::WouldBlock),
            Err(e) => Err(e),
        }
    }
}
impl<F: Framer, const MAX_MSG_SIZE: usize> Drop for FrameReader<F, MAX_MSG_SIZE> {
    fn drop(&mut self) {
        if log_enabled!(log::Level::Debug) {
            debug!("FrameReader::drop {:?}", self.stream_reader);
        }
        match self.stream_reader.shutdown(std::net::Shutdown::Both) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotConnected => {}
            Err(e) => {
                panic!("FrameReader::drop: shutdown error: {}", e);
            }
        }
    }
}
impl<F: Framer, const MAX_MSG_SIZE: usize> Display for FrameReader<F, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {}->{}, fd: {} }}",
            std::any::type_name::<F>()
                .split("::")
                .last()
                .unwrap_or("Unknown"),
            match self.stream_reader.local_addr() {
                Ok(addr) => format!("{:?}", addr),
                Err(_) => "disconnected".to_owned(),
            },
            match self.stream_reader.peer_addr() {
                Ok(addr) => format!("{:?}", addr),
                Err(_) => "disconnected".to_owned(),
            },
            self.stream_reader.as_raw_fd(),
        )
    }
}

#[derive(Debug)]
pub struct FrameWriter {
    pub(crate) stream_writer: mio::net::TcpStream,
}
impl FrameWriter {
    pub fn new(stream: mio::net::TcpStream) -> Self {
        Self {
            stream_writer: stream,
        }
    }
    /// Writes entire frame or no bytes at all to the underlying stream
    /// # Agruments
    ///     * bytes - a slice representing one complete frame
    /// # Result States
    ///     * Ok(WriteStatus::Completed) - all bytes were written to the underlying stream
    ///     * Ok(WriteStatus::NotReady) - zero bytes were written to the underlying stream
    ///     * Err(Box<dyn Error>) - some might be written but eventually write generated Error
    ///
    /// Internally the function will call `write` on the underlying stream until all bytes are written or an error is generated.
    /// This means that if a single `write` successeds the function contrinue to call `write` until all bytes are written or an error is generated.
    /// WriteStatus::NotReady will only return if the first `write` call returns `WouldBlock` and no bytes where written.
    #[inline]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<WriteStatus, Error> {
        let mut residual = bytes;
        while !residual.is_empty() {
            match self.stream_writer.write(residual) {
                // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
                #[rustfmt::skip]
                Ok(EOF) => {
                    let msg = format!("connection reset by peer, residual buf:\n{}", to_hex_pretty(residual));
                    return Err(Error::new(ErrorKind::ConnectionReset, msg));
                }
                Ok(len) => {
                    if len == residual.len() {
                        return Ok(WriteStatus::Completed);
                    } else {
                        residual = &residual[len..];
                        continue;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if bytes.len() == residual.len() {
                        // no bytes where written so Just report back NotReady
                        // println!("write_frame: WouldBlock NotReady");
                        return Ok(WriteStatus::WouldBlock);
                    } else {
                        // println!("write_frame: WouldBlock Continue");
                        // some bytes where written have to finish and report back Completed or Error
                        continue;
                    }
                }
                Err(e) => {
                    let msg = format!("write error: {}, residual:\n{}", e, to_hex_pretty(residual));
                    return Err(Error::new(e.kind(), msg));
                }
            }
        }
        Ok(WriteStatus::Completed)
    }
}
impl Drop for FrameWriter {
    fn drop(&mut self) {
        if log_enabled!(log::Level::Debug) {
            debug!("FrameWriter::drop {:?}", self.stream_writer);
        }
        match self.stream_writer.shutdown(std::net::Shutdown::Both) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotConnected => {}
            Err(e) => {
                panic!("FrameReader::drop: shutdown error: {}", e);
            }
        }
    }
}
impl Display for FrameWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameWriter {{ {}->{}, fd: {} }}",
            match self.stream_writer.local_addr() {
                Ok(addr) => format!("{:?}", addr),
                Err(_) => "disconnected".to_owned(),
            },
            match self.stream_writer.peer_addr() {
                Ok(addr) => format!("{:?}", addr),
                Err(_) => "disconnected".to_owned(),
            },
            self.stream_writer.as_raw_fd(),
        )
    }
}

type FrameProcessor<F, const MAX_MSG_SIZE: usize> = (FrameReader<F, MAX_MSG_SIZE>, FrameWriter);
/// Crates a [FrameReader] and [FrameWriter] from a [std::net::TcpStream] by clonning it and converting the understaing stream to a [mio::net::TcpStream]
/// # Returns
///   * [FrameReader] - a nonblocking FrameReader
///   * [FrameWriter] - a nonblocking FrameWriter
/// # Important
/// If either the [FrameReader] or [FrameWriter] are dropped the underlying stream will be shutdown and all actions on the remainging stream will fail
pub fn into_split_framer<F: Framer, const MAX_MSG_SIZE: usize>(
    stream: std::net::TcpStream,
) -> FrameProcessor<F, MAX_MSG_SIZE> {
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
        mio::net::TcpStream::from_std(reader),
        mio::net::TcpStream::from_std(writer),
    );

    (
        FrameReader::<F, MAX_MSG_SIZE>::new(reader),
        FrameWriter::new(writer),
    )
}

#[cfg(test)]
mod test {
    use std::{
        net::{TcpListener, TcpStream},
        thread::{self, sleep},
        time::{Duration, Instant},
    };

    use crate::prelude_nonblocking::*;

    use bytes::{Bytes, BytesMut};
    use byteserde::utils::hex::to_hex_pretty;
    use links_network_core::prelude::Framer;
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
                    // keep _writer because if you drop it the reader connection will also be closed
                    let (mut reader, _writer) =
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
                            Ok(ReadStatus::WouldBlock) => {
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
        // keep _reader as if you drop it the writer connection will also be closed
        let (_reader, mut writer) =
            into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(TcpStream::connect(addr).unwrap());

        info!("clt: writer: {}", writer);

        let mut frame_send_count = 0_usize;
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            loop {
                match writer.write_frame(send_frame) {
                    Ok(WriteStatus::Completed) => {
                        frame_send_count += 1;
                        break;
                    }
                    Ok(WriteStatus::WouldBlock) => {
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
