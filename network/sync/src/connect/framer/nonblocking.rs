use crate::prelude_nonblocking::{ConId, Framer, RecvStatus, SendStatus};
use bytes::{Bytes, BytesMut};
use byteserde::utils::hex::to_hex_pretty;
use std::{
    fmt::Display,
    io::{Error, ErrorKind, Read, Write},
    net::Shutdown,
};

use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;

use log::{debug, log_enabled};
const EOF: usize = 0;

// TODO evaluate if it is possible to use unsafe set_len on buf then we would not need a MAX_MSG_SIZE generic as it can just be an non const arg to new

#[derive(Debug)]
pub struct FrameReader<F: Framer, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    pub(crate) stream_reader: mio::net::TcpStream,
    buffer: BytesMut,
    phantom: std::marker::PhantomData<F>,
}
impl<F: Framer, const MAX_MSG_SIZE: usize> FrameReader<F, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, reader: mio::net::TcpStream) -> FrameReader<F, MAX_MSG_SIZE> {
        Self {
            con_id,
            stream_reader: reader,
            buffer: BytesMut::with_capacity(MAX_MSG_SIZE),
            phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn read_frame(&mut self) -> Result<RecvStatus<Bytes>, Error> {
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [u8; MAX_MSG_SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        match self.stream_reader.read(&mut buf) {
            Ok(EOF) => {
                self.shutdown(Shutdown::Write, "read_frame EOF"); // remember to shutdown on both exception and on EOF
                if self.buffer.is_empty() {
                    Ok(RecvStatus::Completed(None))
                } else {
                    let msg = format!(
                        "{} FrameReader::read_frame connection reset by peer, residual buf:\n{}",
                        self.con_id,
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
                let msg = format!(
                    "{} FrameReader::read_frame caused by: [{}] residual buf:\n{}",
                    self.con_id,
                    e,
                    to_hex_pretty(&self.buffer[..])
                );

                Err(Error::new(e.kind(), msg))
            }
        }
    }

    /// # Shutdown variants
    /// * [Shutdown::Write] will send TCP FIN flag to the peer and any subsequence [`FrameWriter::write_frame`] will fail with [ErrorKind::BrokenPipe]
    /// * [Shutdown::Read] will not send any TCP flags to the peer but all subsequent reads will [Self::stream_reader] will return Ok(0) and
    /// subsequently [fread_frame] will return [Err(e) if e.kind() == ErrorKind::ConnectionReset]
    #[inline]
    fn shutdown(&mut self, how: Shutdown, reason: &str) {
        if log_enabled!(log::Level::Debug) {
            debug!("{}::shutdown, reason: {}", self, reason);
        }
        match self.stream_reader.shutdown(how) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotConnected => {}
            Err(e) => {
                panic!("{}::shutdown, reason: {}, caused by: [{}]", self, reason, e);
            }
        }
    }
}
impl<F: Framer, const MAX_MSG_SIZE: usize> Drop for FrameReader<F, MAX_MSG_SIZE> {
    /// as a [FrameReader] it wil shutdown the underlying [mio::net::TcpStream] in both directions. This way
    /// the peer connection will recive a TCP FIN flag and and once it reaches the peer [FrameWriter] it will
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
            std::any::type_name::<F>()
                .split("::")
                .last()
                .unwrap_or("Unknown"),
            self.con_id,
            match self.stream_reader.local_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            match self.stream_reader.peer_addr() {
                Ok(_) => "connected",
                Err(_) => "disconnected",
            },
            self.stream_reader.as_raw_fd(),
        )
    }
}

#[derive(Debug)]
pub struct FrameWriter {
    pub(crate) con_id: ConId,
    pub(crate) stream_writer: mio::net::TcpStream,
}
impl FrameWriter {
    pub fn new(con_id: ConId, stream: mio::net::TcpStream) -> Self {
        Self {
            con_id,
            stream_writer: stream,
        }
    }
    /// Writes entire frame or no bytes at all to the underlying stream
    /// # Agruments
    ///    * bytes - a slice representing one complete frame
    /// # Result States
    ///    * [Ok(WriteStatus::Completed)] - all bytes were written to the underlying stream
    ///    * [Ok(WriteStatus::WouldBlock)] - zero bytes were written to the underlying stream
    ///    * [Err(Error)] - some might be written but eventually write generated Error
    ///
    /// Internally the function will c
    /// This means that if a single `write` successeds the function contrinue to call `write` until all bytes are written or an error is generated.
    /// [WriteStatus::WouldBlock] will only return if the first socket `write` fails with [ErrorKind::WouldBlock] and no bytes where written.
    #[inline]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<SendStatus, Error> {
        let mut residual = bytes;
        while !residual.is_empty() {
            match self.stream_writer.write(residual) {
                // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
                #[rustfmt::skip]
                Ok(EOF) => {
                    self.shutdown(Shutdown::Both, "write_frame EOF"); // remember to shutdown on both exception and on EOF
                    let msg = format!("connection reset by peer, residual buf:\n{}", to_hex_pretty(residual));
                    return Err(Error::new(ErrorKind::ConnectionReset, msg));
                }
                Ok(len) => {
                    if len == residual.len() {
                        return Ok(SendStatus::Completed);
                    } else {
                        residual = &residual[len..];
                        continue;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
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
                        "{} FrameWriter::writer_frame caused by: [{}], residual:\n{}",
                        self.con_id,
                        e,
                        to_hex_pretty(residual)
                    );
                    return Err(Error::new(e.kind(), msg));
                }
            }
        }
        Ok(SendStatus::Completed)
    }

    /// Only shutdown the write side of the underlying stream so that the tcp can complete receiving ACKs
    fn shutdown(&mut self, how: Shutdown, reason: &str) {
        if log_enabled!(log::Level::Debug) {
            debug!("{}::shutdown reason: {}", self, reason);
        }
        match self.stream_writer.shutdown(how) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotConnected => {}
            Err(e) => {
                panic!("{}::shutdown reason: {}, caused by: [{}]", self, reason, e);
            }
        }
    }
}
impl Drop for FrameWriter {
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
    mut con_id: ConId,
    stream: std::net::TcpStream,
) -> FrameProcessor<F, MAX_MSG_SIZE> {
    stream
        .set_nonblocking(true)
        .expect("Failed to set_nonblocking on TcpStream");
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
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
        FrameReader::<F, MAX_MSG_SIZE>::new(con_id.clone(), reader),
        FrameWriter::new(con_id, writer),
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
    use links_network_core::{
        fmt_num,
        prelude::{ConId, Framer},
    };
    use links_testing::unittest::setup;
    use log::{error, info};
    use rand::Rng;

    /// # High Level Approach
    /// 1. Spawn FrameReader in a sperate thread
    ///     1. accept connection that will be split into reader & writer
    ///     2. only use reader and read until None or Err
    ///     3. return frame_recv_count upon completion
    /// 2. Create FrameWriter in main thread
    ///     1. the connection will be split into reader & writer
    ///     2. only use writer to write N frames
    ///     3. randomly drop either reader or writer as join FrameReader thread which should succesfully exist in either case
    ///     4. ensure number of frames sent by FrameWriter equals number of frames received by FrameReader
    /// # Notes
    /// * turn on LevelFilter::Debug for addtional logging, it will show drop events
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
        let svc =
            thread::Builder::new()
                .name("Thread-Svc".to_owned())
                .spawn({
                    move || {
                        let listener = TcpListener::bind(addr).unwrap();
                        let (stream, _) = listener.accept().unwrap();
                        // keep _writer because if you drop it the reader connection will also be closed

                        let (mut reader, _writer) =
                            into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(
                                ConId::svc(Some("unittest"), addr, None),
                                stream,
                            );
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
        let stream = TcpStream::connect(addr).unwrap();

        let (reader, mut writer) = into_split_framer::<MsgFramer, TEST_SEND_FRAME_SIZE>(
            ConId::clt(Some("unittest"), None, addr),
            stream,
        );

        info!("clt: writer: {}", writer);

        let mut frame_send_count = 0_usize;
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            loop {
                match writer.write_frame(send_frame) {
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

        if rand::thread_rng().gen_range(1..=2) % 2 == 0 {
            info!("dropping writer");
            drop(writer);
        } else {
            info!("dropping reader");
            drop(reader);
        }
        let frame_recv_count = svc.join().unwrap();
        info!(
            "frame_send_count: {}, frame_recv_count: {}",
            fmt_num!(frame_send_count),
            fmt_num!(frame_recv_count)
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
