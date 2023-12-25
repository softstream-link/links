//! This module contains a blocking `paired` [FrameReader] and [FrameWriter] which are designed to be used in separate threads.
//! where each thread is only doing either reading or writing to the underlying [std::net::TcpStream].
//!
//! # Note
//! The underlying [std::net::TcpStream] is cloned and therefore share a single underlying network socket.
//!
//! # Example
//! ```
//! use links_blocking::prelude::*;
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

use crate::prelude::{ConId, Framer};
use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use links_core::asserted_short_name;
use log::{debug, log_enabled};
use std::fmt::Display;
use std::io::{ErrorKind, Read, Write};
use std::mem::MaybeUninit;
use std::net::Shutdown;
use std::{io::Error, net::TcpStream};

#[cfg(target_family = "unix")]
#[inline]
fn fd(stream: &TcpStream) -> std::os::fd::RawFd {
    use std::os::fd::AsRawFd;
    stream.as_raw_fd()
}
#[cfg(target_family = "windows")]
#[inline]
fn fd(stream: &TcpStream) -> &'static str {
    "windows"
}

const EOF: usize = 0;

/// Represents an abstraction for reading exactly oen frame from the [TcpStream].
/// Each call to the [Self::read_frame] will issue a [Read::read] system call on the underlying [TcpStream].
/// which will capture any bytes read into internal accumulator implemented as a [BytesMut]. This internal buffer will be
/// passed to the generic impl of [Framer::get_frame] where it is user's responsibility to  inspect the buffer and split off a single frame.
///
/// # Generic Parameters
///  * `F` - A type that implements the [Framer] trait. This is used to split off a single frame from the internal buffer.
///  * `MAX_MSG_SIZE` - The maximum size of a single frame. This is used to pre-allocate the internal buffer.
/// Set this number to the maximum size of a single frame for your protocol.
///  
#[derive(Debug)]
pub struct FrameReader<F: Framer, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) stream_reader: TcpStream,
    buffer: BytesMut,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer, const MAX_MSG_SIZE: usize> FrameReader<F, MAX_MSG_SIZE> {
    /// Creates a new instance of [FrameReader]
    /// # Arguments
    /// * `con_id` - [ConId] a unique identifier for the connection and used for logging
    /// * `reader` - [TcpStream] the underlying stream that will be used for reading
    pub fn new(con_id: ConId, reader: TcpStream) -> FrameReader<F, MAX_MSG_SIZE> {
        Self {
            con_id,
            stream_reader: reader,
            buffer: BytesMut::with_capacity(MAX_MSG_SIZE),
            phantom: std::marker::PhantomData,
        }
    }

    /// Reads `exactly one frame` from the underlying [TcpStream] and returns it as a [Some(Bytes)] or [None] if the connection was closed.
    ///
    /// # Note
    /// If the [FrameWriter] `pair` is dropped then this method will return a [Ok(None)].
    #[inline]
    pub fn read_frame(&mut self) -> Result<Option<Bytes>, Error> {
        loop {
            if let Some(bytes) = F::get_frame(&mut self.buffer) {
                return Ok(Some(bytes));
            } else {
                #[allow(clippy::uninit_assumed_init)]
                let mut buf: [u8; MAX_MSG_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };
                match self.stream_reader.read(&mut buf) {
                    Ok(EOF) => {
                        self.shutdown(Shutdown::Write, "read_frame EOF");
                        if self.buffer.is_empty() {
                            return Ok(None);
                        } else {
                            let msg = format!(
                                "{} {}::read_frame connection reset by peer, residual buf:\n{}",
                                self.con_id,
                                asserted_short_name!("FrameReader", Self),
                                to_hex_pretty(&self.buffer[..])
                            );
                            return Err(Error::new(std::io::ErrorKind::ConnectionReset, msg));
                        }
                    }
                    Ok(len) => {
                        self.buffer.extend_from_slice(&buf[..len]);
                        continue; // more bytes added, try to get a frame again
                    }
                    Err(e) => {
                        self.shutdown(Shutdown::Write, "read_frame error");
                        let msg = format!("{} {}::read_frame caused by: [{}] residual buf:\n{}", self.con_id, asserted_short_name!("FrameReader", Self), e, to_hex_pretty(&self.buffer[..]));
                        return Err(Error::new(e.kind(), msg));
                    }
                }
            }
        }
    }
    #[inline]
    fn shutdown(&mut self, how: Shutdown, reason: &str) {
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
    /// Will shutdown the underlying [std::net::TcpStream] in both directions. This way
    /// the `peer` connection will receive a TCP FIN flag and and once it reaches the `peer` [FrameWriter] it will
    /// get a [ErrorKind::BrokenPipe] error which in turn shall issue a [Shutdown::Write]
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "drop")
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

/// Represents an abstraction for writing a single frame to the underlying [TcpStream].
#[derive(Debug)]
pub struct FrameWriter {
    pub(crate) con_id: ConId,
    pub(crate) stream_writer: TcpStream,
}
impl FrameWriter {
    /// Creates a new instance of [FrameWriter]
    /// # Arguments
    /// * `con_id` - [ConId] a unique identifier for the connection and used for logging
    /// * `stream` - [TcpStream] the underlying stream that will be used for writing
    pub fn new(con_id: ConId, stream: TcpStream) -> Self {
        Self { con_id, stream_writer: stream }
    }
    /// Writes a single frame to the underlying [TcpStream].
    ///
    /// # Note
    /// If the [FrameReader] `pair` is dropped then this method will return a [Err(ErrorKind::BrokenPipe)] error.
    #[inline]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self.stream_writer.write_all(bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                self.shutdown(Shutdown::Write, "write_frame error");
                let msg = format!("{} FrameWriter::write_frame caused by: [{}]", self.con_id, e);
                Err(Error::new(e.kind(), msg))
            }
        }
    }

    /// Shuts down the underlying [std::net::TcpStream] in the specified direction.
    fn shutdown(&mut self, how: Shutdown, reason: &str) {
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
    /// Will shutdown the underlying [std::net::TcpStream] in both directions.
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "drop")
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

/// Creates a `paired` [FrameReader] and [FrameWriter] from a [std::net::TcpStream] by cloning
///
/// # Returns a tuple with
///   * [FrameReader] - a blocking FrameReader
///   * [FrameWriter] - a blocking FrameWriter
///
/// # Important
/// If either the [FrameReader] or [FrameWriter] are dropped the underlying stream will be shutdown and all actions on the remaining `pair` will fail
pub fn into_split_framer<F: Framer, const MAX_MSG_SIZE: usize>(mut con_id: ConId, stream: TcpStream) -> FrameProcessor<F, MAX_MSG_SIZE> {
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
    let (reader, writer) = (stream.try_clone().expect("Failed to try_clone TcpStream for FrameReader"), stream);
    (FrameReader::<F, MAX_MSG_SIZE>::new(con_id.clone(), reader), FrameWriter::new(con_id, writer))
}

#[cfg(test)]
mod test {

    use std::{
        io::ErrorKind,
        net::{TcpListener, TcpStream},
        thread::{self, sleep},
        time::{Duration, Instant},
    };

    use crate::prelude::*;

    use byteserde::utils::hex::to_hex_pretty;
    use links_core::{fmt_num, prelude::ConId, unittest::setup};

    use log::{error, info};
    use rand::Rng;

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
        info!("send_frame: \n{}", to_hex_pretty(send_frame));

        let addr = setup::net::rand_avail_addr_port();

        // CONFIGURE svc
        let svc = thread::Builder::new()
            .name("Thread-Svc".to_owned())
            .spawn({
                move || {
                    let listener = TcpListener::bind(addr).unwrap();
                    let (stream, _) = listener.accept().unwrap();
                    let (mut svc_reader, _svc_writer) = into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(ConId::svc(Some("unittest"), addr, None), stream);
                    info!("svc: reader: {}", svc_reader);
                    let mut frame_recv_count = 0_usize;
                    loop {
                        let res = svc_reader.read_frame();
                        match res {
                            Ok(frame) => {
                                if let None = frame {
                                    info!("svc: read_frame is None, client closed connection");
                                    break;
                                } else {
                                    frame_recv_count += 1;
                                }
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
        let (mut clt_reader, mut clt_writer) = into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(ConId::clt(Some("unittest"), None, addr), TcpStream::connect(addr).unwrap());

        info!("clt: {}", clt_writer);

        let mut frame_send_count = 0_usize;
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            clt_writer.write_frame(send_frame).unwrap();
            frame_send_count += 1;
        }
        let elapsed = start.elapsed();

        // drop either clt_reader or clt_writer and validate that the `pair` is acting correction
        if rand::thread_rng().gen_range(1..=2) % 2 == 0 {
            info!("dropping clt_writer");
            drop(clt_writer);
            let opt = clt_reader.read_frame().unwrap();
            info!("clt_reader.read_frame() opt: {:?}", opt);
            assert_eq!(opt, None);
        } else {
            info!("dropping clt_reader");
            drop(clt_reader);
            let err = clt_writer.write_frame(send_frame).unwrap_err();
            info!("clt_writer.write_frame() err: {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }
        let frame_recv_count = svc.join().unwrap();

        info!("frame_send_count: {}, frame_recv_count: {}", fmt_num!(frame_send_count), fmt_num!(frame_recv_count));
        info!("per send elapsed: {:?}, total elapsed: {:?} ", elapsed / WRITE_N_TIMES as u32, elapsed);
        assert_eq!(frame_send_count, frame_recv_count);
        assert_eq!(frame_send_count, WRITE_N_TIMES);
    }
}
