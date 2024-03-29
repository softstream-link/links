//! This module contains a non blocking `paired` [FrameReader] and [FrameWriter] which are designed to be used in separate threads,
//! where each thread is only doing either reading or writing to the underlying [mio::net::TcpStream].
//!
//! # Note
//!
//!  The underlying [mio::net::TcpStream] is cloned and therefore share a single underlying network socket.
//!
//! # Example
//! ```
//! use links_nonblocking::prelude::*;
//!
//! const FRAME_SIZE: usize = 128;
//!
//! let addr = "127.0.0.1:8080";
//!
//! let svc_listener = std::net::TcpListener::bind(addr).unwrap();
//!
//! let clt_stream = std::net::TcpStream::connect(addr).unwrap();
//! let (clt_reader, clt_writer) = into_split_framer::<FixedSizeFramer<FRAME_SIZE>, FRAME_SIZE>(
//!         ConId::clt(Some("unittest"), None, addr),
//!         clt_stream,
//!     );
//!
//! let svc_stream = svc_listener.accept().unwrap().0;
//! let (svc_reader, svc_writer) = into_split_framer::<FixedSizeFramer<FRAME_SIZE>, FRAME_SIZE>(
//!         ConId::svc(Some("unittest"), addr, None),
//!         svc_stream,
//!     );
//!
//! drop(clt_reader);
//! drop(clt_writer);
//! drop(svc_reader);
//! drop(svc_writer);
//! drop(svc_listener);
//!
//! // Note:
//!     // paired
//!         // clt_reader & clt_writer
//!         // svc_reader & svc_writer
//!     // peers
//!         // clt_reader & svc_writer
//!         // svc_reader & clt_writer
//! ```

use crate::prelude::{ConId, Framer, RecvStatus, SendStatus};
use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use links_core::asserted_short_name;
use mio::net::TcpStream;
use std::mem::MaybeUninit;
use std::{
    fmt::Display,
    io::{Error, ErrorKind, Read, Write},
    net::Shutdown,
};

#[cfg(target_family = "unix")]
#[inline]
fn fd(stream: &TcpStream) -> std::os::fd::RawFd {
    use std::os::fd::AsRawFd;
    stream.as_raw_fd()
}
#[cfg(target_family = "windows")]
#[inline]
fn fd(stream: &TcpStream) -> std::os::windows::io::RawSocket {
    use std::os::windows::io::AsRawSocket;
    stream.as_raw_socket()
}

use log::{debug, log_enabled};
const EOF: usize = 0;

/// Represents an abstraction for reading exactly one frame from the [TcpStream].
/// Each call to [Self::read_frame] will issue a [Read::read] system call on the underlying [TcpStream]
/// which will capture any bytes read into internal accumulator implemented as [BytesMut]. This internal buffer will be
/// passed to the generic impl of [Framer::get_frame] where it is user's responsibility to inspect the buffer and split off a single frame.
///
/// # Generic Parameters
///  * `F` - a type that implements [Framer] trait. This trait is used to split off a single frame from the internal buffer
///  * `MAX_MSG_SIZE` - a const generic that represents the maximum size of a single frame. This is used to preallocate the internal buffer.
/// Set this number to the maximum size of a single frame for your protocol.
#[derive(Debug)]
pub struct FrameReader<F: Framer, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) stream_reader: TcpStream,
    buffer: BytesMut,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer, const MAX_MSG_SIZE: usize> FrameReader<F, MAX_MSG_SIZE> {
    /// Constructs a new instance of [FrameReader]
    /// # Arguments
    /// * `con_id` - [ConId] a unique identifier for the connection and used for logging
    /// * `reader` - [mio::net::TcpStream] the underlying stream that will be used for reading
    pub fn new(con_id: ConId, reader: mio::net::TcpStream) -> FrameReader<F, MAX_MSG_SIZE> {
        Self {
            con_id,
            stream_reader: reader,
            buffer: BytesMut::with_capacity(MAX_MSG_SIZE),
            phantom: std::marker::PhantomData,
        }
    }

    /// Reads `exactly one frame` from the underlying [TcpStream], see [RecvStatus] for more details on the meaning of
    /// each variant in the successful scenario.
    /// # Note
    /// If the [FrameWriter] `pair` is dropped this method will return [RecvStatus::Completed(None)]
    #[inline(always)]
    pub fn read_frame(&mut self) -> Result<RecvStatus<Bytes>, Error> {
        if let Some(bytes) = F::get_frame(&mut self.buffer) {
            return Ok(RecvStatus::Completed(Some(bytes)));
        }
        // TODO evaluate if it is possible to use unsafe set_len on buf then we would not need a MAX_MSG_SIZE generic as it can just be an non const arg to new
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [u8; MAX_MSG_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.stream_reader.read(&mut buf) {
            Ok(EOF) => {
                // key to shutdown using Write as this will
                self.shutdown(Shutdown::Write, "read_frame EOF"); // remember to shutdown on both exception and on EOF
                if self.buffer.is_empty() {
                    Ok(RecvStatus::Completed(None))
                } else {
                    let msg = format!(
                        "{} {}::read_frame connection reset by peer, residual buf:\n{}",
                        self.con_id,
                        asserted_short_name!("FrameReader", Self),
                        to_hex_pretty(&self.buffer[..])
                    );
                    Err(Error::new(ErrorKind::ConnectionReset, msg))
                }
            }
            Ok(len) => {
                self.buffer.extend_from_slice(&buf[..len]);
                if let Some(bytes) = F::get_frame(&mut self.buffer) {
                    Ok(RecvStatus::Completed(Some(bytes)))
                } else {
                    Ok(RecvStatus::WouldBlock)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(RecvStatus::WouldBlock),
            Err(e) => {
                self.shutdown(Shutdown::Write, "read_frame error"); // remember to shutdown on both exception and on EOF
                let buf = format!(
                    "len: {} content: {}",
                    self.buffer.len(),
                    if !self.buffer.is_empty() { format!("\n{}", to_hex_pretty(&self.buffer[..])) } else { "Empty".to_owned() }
                );
                let msg = format!("{} {}::read_frame caused by: [{}] residual buf {}", self.con_id, asserted_short_name!("FrameReader", Self), e, buf);
                Err(Error::new(e.kind(), msg))
            }
        }
    }

    /// Shuts down the underlying [TcpStream] in the specified direction.
    /// # Note side effects of each variant below
    ///  * [Shutdown::Write] will send TCP FIN flag to the peer, as a result all subsequent `paired` [FrameWriter::write_frame] will fail with [ErrorKind::BrokenPipe]
    ///  * [Shutdown::Read] will `NOT` send any TCP flags to the peer, however, as a result all subsequent [Self::read_frame] will return [Ok(0)].
    /// This variant will also cause all `peer` [FramerWriter::write_frame] to generate [std::io::Error] of [ErrorKind::ConnectionReset]
    #[inline(always)]
    pub(crate) fn shutdown(&mut self, how: Shutdown, reason: &str) {
        match self.stream_reader.shutdown(how) {
            Ok(_) => {
                if log_enabled!(log::Level::Debug) {
                    debug!("{}::shutdown how: {:?}, reason: {}", self, how, reason);
                }
            }
            Err(e) if e.kind() == ErrorKind::NotConnected => {
                if log_enabled!(log::Level::Debug) {
                    debug!("{}::shutdown while disconnected how: {:?}, reason: {}", self, how, reason);
                }
            }
            Err(e) => {
                panic!("{}::shutdown how: {:?}, reason: {}, caused by: [{}]", self, how, reason, e);
            }
        }
    }
}
impl<F: Framer, const MAX_MSG_SIZE: usize> Drop for FrameReader<F, MAX_MSG_SIZE> {
    /// Will shutdown the underlying [mio::net::TcpStream] in both directions. This way
    /// the `peer` connection will receive a TCP FIN flag and and once it reaches the `peer` [FrameWriter] it will
    /// get a [ErrorKind::BrokenPipe] error which in turn shall issue a [Shutdown::Write]
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "FrameReader::drop")
    }
}
impl<F: Framer, const MAX_MSG_SIZE: usize> Display for FrameReader<F, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameReader<{}> {{ {}, addr: {}, peer: {}, fd: {} }}",
            std::any::type_name::<F>().split("::").last().unwrap_or("Unknown"),
            self.con_id,
            match self.stream_reader.local_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            match self.stream_reader.peer_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            fd(&self.stream_reader),
        )
    }
}

/// Represents an abstraction for writing exactly one frame to the non blocking underlying [mio::net::TcpStream]
#[derive(Debug)]
pub struct FrameWriter {
    pub(crate) con_id: ConId,
    pub(crate) stream_writer: mio::net::TcpStream,
}
impl FrameWriter {
    /// Constructs a new instance of [FrameWriter]
    pub fn new(con_id: ConId, stream: mio::net::TcpStream) -> Self {
        Self { con_id, stream_writer: stream }
    }
    /// Writes `entire` frame or `no` bytes at all to the underlying stream, see [SendStatus] for more details on the meaning of
    /// each variant in the successful scenario.
    ///
    /// # Arguments
    ///    * bytes - a slice representing one complete frame
    ///
    /// # Important
    /// The function will internally issue a [Write::write] system call repeatedly on the underlying [mio::net::TcpStream]
    /// until all of the bytes are written, while `busy waiting` on the socket if write returns [ErrorKind::WouldBlock].
    ///
    /// However, if an only if, the first call to [Write::write] returns [ErrorKind::WouldBlock] and no bytes where written
    /// to the underlying socket, the method will return immediately with [Ok(SendStatus::WouldBlock)].
    ///
    /// # Note
    /// If the [FrameReader] `pair` is dropped this method will return [Err(ErrorKind::BrokenPipe)]
    #[inline(always)]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<SendStatus, Error> {
        let mut residual = bytes;
        while !residual.is_empty() {
            match self.stream_writer.write(residual) {
                // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
                Ok(EOF) => {
                    self.shutdown(Shutdown::Both, "write_frame EOF"); // remember to shutdown on both exception and on EOF
                    let msg = format!("{} {}::write_frame connection reset by peer, residual buf:\n{}", self.con_id, asserted_short_name!("FrameWriter", Self), to_hex_pretty(residual));
                    return Err(Error::new(ErrorKind::ConnectionReset, msg));
                }
                Ok(len) => {
                    if len == residual.len() {
                        self.stream_writer.flush().unwrap();
                        return Ok(SendStatus::Completed);
                    } else {
                        residual = &residual[len..];
                        continue;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if bytes.len() == residual.len() {
                        // no bytes where written so Just report back NotReady
                        return Ok(SendStatus::WouldBlock);
                    } else {
                        // some bytes where written have to finish and report back Completed or Error
                        continue;
                    }
                }
                Err(e) => {
                    self.shutdown(Shutdown::Both, "write_frame error"); // remember to shutdown on both exception and on EOF
                    let msg = format!(
                        "{} {}::write_frame caused by: [{}], residual len: {}\n{}",
                        self.con_id,
                        asserted_short_name!("FrameWriter", Self),
                        e,
                        residual.len(),
                        to_hex_pretty(residual)
                    );
                    return Err(Error::new(e.kind(), msg));
                }
            }
        }
        // Shall never get here as it would indicate that we are trying to send an empty slice
        Ok(SendStatus::Completed)
    }

    /// Shuts down the underlying [mio::net::TcpStream] in the specified direction.
    pub(crate) fn shutdown(&mut self, how: Shutdown, reason: &str) {
        match self.stream_writer.shutdown(how) {
            Ok(_) => {
                if log_enabled!(log::Level::Debug) {
                    debug!("{}::shutdown how: {:?}, reason: {}", self, how, reason);
                }
            }
            Err(e) if e.kind() == ErrorKind::NotConnected => {
                if log_enabled!(log::Level::Debug) {
                    debug!("{}::shutdown while disconnected how: {:?}, reason: {}", self, how, reason);
                }
            }
            Err(e) => {
                panic!("{}::shutdown how: {:?}, reason: {}, caused by: [{}]", self, how, reason, e);
            }
        }
    }
}
impl Drop for FrameWriter {
    /// Will shutdown the underlying [mio::net::TcpStream] in both directions.
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "FrameWriter::drop")
    }
}
impl Display for FrameWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FrameWriter {{ {}, addr: {}, peer: {}, fd: {} }}",
            self.con_id,
            match self.stream_writer.local_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            match self.stream_writer.peer_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            fd(&self.stream_writer),
        )
    }
}

type FrameProcessor<F, const MAX_MSG_SIZE: usize> = (FrameReader<F, MAX_MSG_SIZE>, FrameWriter);

/// Creates a `paired` [FrameReader] and [FrameWriter] from a [std::net::TcpStream] by cloning it and converting
/// the underlying stream to [mio::net::TcpStream]
///
/// # Returns a tuple with
///   * [FrameReader] - a nonblocking FrameReader
///   * [FrameWriter] - a nonblocking FrameWriter
///
/// # Important
/// If either the [FrameReader] or [FrameWriter] are dropped the underlying stream will be shutdown and all actions on the remaining `pair` will fail
pub fn into_split_framer<F: Framer, const MAX_MSG_SIZE: usize>(mut con_id: ConId, stream: std::net::TcpStream) -> FrameProcessor<F, MAX_MSG_SIZE> {
    stream.set_nonblocking(true).expect("Failed to set_nonblocking on TcpStream");
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
    let (reader, writer) = (stream.try_clone().expect("Failed to try_clone TcpStream for FrameReader"), stream);

    let (reader, writer) = (mio::net::TcpStream::from_std(reader), mio::net::TcpStream::from_std(writer));

    (FrameReader::<F, MAX_MSG_SIZE>::new(con_id.clone(), reader), FrameWriter::new(con_id, writer))
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use byteserde::utils::hex::to_hex_pretty;
    use links_core::{fmt_num, prelude::ConId, unittest::setup};
    use log::{error, info};
    use rand::Rng;
    use std::{
        net::{TcpListener, TcpStream},
        thread::{self, sleep},
        time::{Duration, Instant},
    };

    /// # High Level Approach
    /// 1. Spawn Svc FrameReader in a separate thread
    ///     1. accept connection that will be split into reader & writer
    ///     2. only use reader and read until None or Err
    ///     3. return frame_recv_count upon completion
    /// 2. Create Clt FrameWriter in main thread
    ///     1. the connection will be split into reader & writer
    ///     2. only use writer to write N frames
    ///     3. randomly drop either reader or writer as join FrameReader thread which should successfully exist in either case
    ///     4. ensure number of frames sent by FrameWriter equals number of frames received by FrameReader
    /// # Notes
    /// * turn on LevelFilter::Debug for additional logging, it will show drop events
    #[test]
    fn test_reader() {
        setup::log::configure_level(log::LevelFilter::Info);
        const TEST_SEND_FRAME_SIZE: usize = 128;
        const WRITE_N_TIMES: usize = 100_000;
        pub type MsgFramer = FixedSizeFramer<TEST_SEND_FRAME_SIZE>;

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

                    let (mut reader, _writer) = into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(ConId::svc(Some("unittest"), addr, None), stream);
                    info!("svc: reader: {}", reader);
                    let mut frame_recv_count = 0_usize;
                    loop {
                        let res = reader.read_frame();
                        match res {
                            Ok(RecvStatus::Completed(None)) => {
                                info!("svc: read_frame is None, client closed connection");
                                break;
                            }
                            Ok(RecvStatus::Completed(Some(recv_frame))) => {
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
                            Ok(RecvStatus::WouldBlock) => {
                                continue; // try reading again
                            }
                            Err(e) => {
                                error!("Svc read_frame error: {}", e.to_string());
                                break;
                            }
                        }
                    }
                    frame_recv_count
                }
            })
            .unwrap();

        sleep(Duration::from_millis(100)); // allow the spawned to bind

        // CONFIGURE clt
        let stream = TcpStream::connect(addr).unwrap();

        let (mut clt_reader, mut clt_writer) = into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(ConId::clt(Some("unittest"), None, addr), stream);

        info!("clt: writer: {}", clt_writer);

        let mut frame_send_count = 0_usize;
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            loop {
                match clt_writer.write_frame(send_frame) {
                    Ok(SendStatus::Completed) => {
                        frame_send_count += 1;
                        break;
                    }
                    Ok(SendStatus::WouldBlock) => {
                        continue;
                    }
                    Err(e) => {
                        panic!("clt write_frame error: {}", e.to_string());
                    }
                }
            }
        }
        let elapsed = start.elapsed();

        // drop either clt_reader or clt_writer and validate that the `pair` is acting correction
        let drop_scenario = rand::thread_rng().gen_range(1..=2);
        if drop_scenario % 2 == 0 {
            info!("dropping clt_writer");
            drop(clt_writer);
            let status = clt_reader.read_frame().unwrap();
            info!("clt_reader.read_frame() status: {:?}", status);
            assert_eq!(status, RecvStatus::Completed(None));
        } else {
            info!("dropping clt_reader");
            drop(clt_reader);
            let err = clt_writer.write_frame(send_frame).unwrap_err();
            info!("clt_writer.write_frame() err: {}", err);
            assert_error_kind_on_target_family!(err, std::io::ErrorKind::BrokenPipe);
        }
        let frame_recv_count = svc.join().unwrap();
        info!("frame_send_count: {}, frame_recv_count: {}", fmt_num!(frame_send_count), fmt_num!(frame_recv_count));
        info!("per send elapsed: {:?}, total elapsed: {:?} ", elapsed / WRITE_N_TIMES as u32, elapsed);
        assert_eq!(frame_send_count, frame_recv_count);
        assert_eq!(frame_send_count, WRITE_N_TIMES);
    }
}
