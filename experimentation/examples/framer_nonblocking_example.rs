use bytes::{Bytes, BytesMut};
use criterion::{black_box, Criterion};
use links_core::prelude::FixedSizeFramer;
use std::fmt::Debug;
use std::io::{Error, ErrorKind, Read, Write};
use std::net::Shutdown;
use std::{
    net::{TcpListener, TcpStream},
    thread::{self, sleep},
    time::Duration,
};

const EOF: usize = 0;
const BENCH_MAX_FRAME_SIZE: usize = 128;
pub trait Framer: Debug {
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes>;
}
#[derive(Debug)]
pub struct FrameReader {
    stream_reader: mio::net::TcpStream,
    buffer: BytesMut,
}
#[derive(Debug, PartialEq)]
pub enum RecvStatus<T> {
    Completed(Option<T>),
    WouldBlock,
}
#[derive(Debug, PartialEq)]
pub enum SendStatus {
    Completed,
    WouldBlock,
}
impl FrameReader {
    pub fn new(reader: mio::net::TcpStream) -> FrameReader {
        Self {
            stream_reader: reader,
            buffer: BytesMut::with_capacity(BENCH_MAX_FRAME_SIZE),
        }
    }
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        if bytes.len() < BENCH_MAX_FRAME_SIZE {
            return None;
        } else {
            let frame = bytes.split_to(BENCH_MAX_FRAME_SIZE);
            return Some(frame.freeze());
        }
    }

    #[inline]
    pub fn read_frame(&mut self) -> Result<RecvStatus<Bytes>, Error> {
        #[allow(clippy::uninit_assumed_init)]
        let mut buf: [u8; BENCH_MAX_FRAME_SIZE] = [0; BENCH_MAX_FRAME_SIZE];
        //= unsafe { MaybeUninit::uninit().assume_init() };
        match self.stream_reader.read(&mut buf) {
            Ok(EOF) => {
                self.shutdown(Shutdown::Write, "read_frame EOF"); // remember to shutdown on both exception and on EOF
                if self.buffer.is_empty() {
                    Ok(RecvStatus::Completed(None))
                } else {
                    let msg = format!("FrameReader::read_frame connection reset by peer, residual buf:\n{:x?}", &self.buffer[..]);
                    Err(Error::new(ErrorKind::ConnectionReset, msg))
                }
            }
            Ok(len) => {
                self.buffer.extend_from_slice(&buf[..len]);
                if let Some(bytes) = Self::get_frame(&mut self.buffer) {
                    Ok(RecvStatus::Completed(Some(bytes)))
                } else {
                    Ok(RecvStatus::WouldBlock)
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(RecvStatus::WouldBlock),
            Err(e) => {
                self.shutdown(Shutdown::Write, "read_frame error"); // remember to shutdown on both exception and on EOF
                let msg = format!("rameReader::read_frame caused by: [{}] residual buf:\n{:?}", e, &self.buffer[..]);

                Err(Error::new(e.kind(), msg))
            }
        }
    }

    #[inline]
    fn shutdown(&mut self, how: Shutdown, reason: &str) {
        match self.stream_reader.shutdown(how) {
            Ok(_) => {
                println!("{:?}::shutdown how: {:?}, reason: {}", self, how, reason);
            }
            Err(e) if e.kind() == ErrorKind::NotConnected => {
                println!("{:?}::shutdown while diconnected how: {:?}, reason: {}", self, how, reason);
            }
            Err(e) => {
                panic!("{:?}::shutdown how: {:?}, reason: {}, caused by: [{}]", self, how, reason, e);
            }
        }
    }
}
impl Drop for FrameReader {
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "drop")
    }
}

pub type BenchMsgFramer = FixedSizeFramer<BENCH_MAX_FRAME_SIZE>;

#[derive(Debug)]
pub struct FrameWriter {
    stream_writer: mio::net::TcpStream,
}
impl FrameWriter {
    pub fn new(stream: mio::net::TcpStream) -> Self {
        Self { stream_writer: stream }
    }
    #[inline]
    pub fn write_frame(&mut self, bytes: &[u8]) -> Result<SendStatus, Error> {
        let mut residual = bytes;
        while !residual.is_empty() {
            match self.stream_writer.write(residual) {
                // note: can't use write_all https://github.com/rust-lang/rust/issues/115451
                Ok(EOF) => {
                    self.shutdown(Shutdown::Both, "write_frame EOF"); // remember to shutdown on both exception and on EOF
                    let msg = format!("connection reset by peer, residual buf:\n{:?}", residual);
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
                    let msg = format!("FrameWriter::writer_frame caused by: [{}], residual len: {}\n{:?}", e, residual.len(), residual);
                    return Err(Error::new(e.kind(), msg));
                }
            }
        }
        Ok(SendStatus::Completed)
    }

    fn shutdown(&mut self, how: Shutdown, reason: &str) {
        match self.stream_writer.shutdown(how) {
            Ok(_) => {
                println!("{:?}::shutdown how: {:?}, reason: {}", self, how, reason);
            }
            Err(e) if e.kind() == ErrorKind::NotConnected => {
                println!("{:?}::shutdown while disconnected how: {:?}, reason: {}", self, how, reason);
            }
            Err(e) => {
                panic!("{:?}::shutdown how: {:?}, reason: {}, caused by: [{}]", self, how, reason, e);
            }
        }
    }
}
impl Drop for FrameWriter {
    fn drop(&mut self) {
        self.shutdown(Shutdown::Both, "drop")
    }
}
pub fn into_split_framer(stream: std::net::TcpStream) -> (FrameReader, FrameWriter) {
    stream.set_nonblocking(true).expect("Failed to set_nonblocking on TcpStream");

    let (reader, writer) = (stream.try_clone().expect("Failed to try_clone TcpStream for FrameReader"), stream);

    let (reader, writer) = (mio::net::TcpStream::from_std(reader), mio::net::TcpStream::from_std(writer));

    (FrameReader::new(reader), FrameWriter::new(writer))
}

fn recv_random_frame_with_creterion() {
    let mut c = Criterion::default();

    let send_frame = [1_u8; BENCH_MAX_FRAME_SIZE].as_slice();
    let addr = "0.0.0.0:8080";

    // CONFIGURE svc
    let svc_writer_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (svc_reader, mut svc_writer) = into_split_framer(stream);
                println!("svc_reader: {:?}, svc_writer: {:?}", svc_reader, svc_writer);
                let mut frame_send_count = 0_usize;
                loop {
                    match svc_writer.write_frame(send_frame) {
                        Ok(SendStatus::Completed) => {
                            frame_send_count += 1;
                        }
                        Ok(SendStatus::WouldBlock) => {
                            continue;
                        }
                        Err(e) => {
                            println!("Svc write_frame, expected error: {}", e); // not error as client will stop reading and drop
                            break;
                        }
                    }
                }
                frame_send_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (mut clt_reader, clt_writer) = into_split_framer(TcpStream::connect(addr).unwrap());
    println!("clt_reader: {:?}, clt_writer: {:?}", clt_reader, clt_writer);

    let mut frame_recv_count = 0_usize;
    c.bench_function("read", |b| {
        b.iter(|| {
            black_box({
                loop {
                    match clt_reader.read_frame() {
                        Ok(RecvStatus::Completed(Some(_))) => {
                            frame_recv_count += 1;
                            break;
                        }
                        Ok(RecvStatus::WouldBlock) => {
                            continue;
                        }
                        Ok(RecvStatus::Completed(None)) => {
                            panic!("clt: read_frame is None, server closed connection");
                        }
                        Err(e) => {
                            panic!("clt: read_frame error: {:?}", e);
                        }
                    }
                }
            })
        })
    });
    c.final_summary();
    // clt_reader.stream_reader.shutdown(Shutdown::Both).unwrap();
    drop(clt_reader); // this will allow svc.join to complete
                      // drop(_clt_writer); // TODO critical github hangs unless write dropped, finish pod ubuntu testing with tshark and remove this drop
    let frame_send_count = svc_writer_jh.join().unwrap();
    println!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        frame_send_count,
        frame_recv_count,
        frame_send_count - frame_recv_count,
    );

    assert!(frame_send_count > frame_recv_count);
}

fn main() {
    recv_random_frame_without_creterion();
    recv_random_frame_with_creterion();
}

fn recv_random_frame_without_creterion() {
    let send_frame = [1_u8; BENCH_MAX_FRAME_SIZE].as_slice();
    let addr = "0.0.0.0:8080";

    // CONFIGURE svc
    let svc_writer_jh = thread::Builder::new()
        .name("Thread-Svc".to_owned())
        .spawn({
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (svc_reader, mut svc_writer) = into_split_framer(stream);
                println!("svc_reader: {:?}, svc_writer: {:?}", svc_reader, svc_writer);
                let mut frame_send_count = 0_usize;
                loop {
                    match svc_writer.write_frame(send_frame) {
                        Ok(SendStatus::Completed) => {
                            frame_send_count += 1;
                        }
                        Ok(SendStatus::WouldBlock) => {
                            continue;
                        }
                        Err(e) => {
                            println!("Svc write_frame, expected error: {}", e); // not error as client will stop reading and drop
                            break;
                        }
                    }
                }
                frame_send_count
            }
        })
        .unwrap();

    sleep(Duration::from_millis(100)); // allow the spawned to bind

    // CONFIGUR clt
    let (mut clt_reader, clt_writer) = into_split_framer(TcpStream::connect(addr).unwrap());
    println!("clt_reader: {:?}, clt_writer: {:?}", clt_reader, clt_writer);

    let mut frame_recv_count = 0_usize;
    for _ in 0..10_000_000 {
        loop {
            match clt_reader.read_frame() {
                Ok(RecvStatus::Completed(Some(_))) => {
                    frame_recv_count += 1;
                    break;
                }
                Ok(RecvStatus::WouldBlock) => {
                    continue;
                }
                Ok(RecvStatus::Completed(None)) => {
                    panic!("clt: read_frame is None, server closed connection");
                }
                Err(e) => {
                    panic!("clt: read_frame error: {:?}", e);
                }
            }
        }
    }

    // clt_reader.stream_reader.shutdown(Shutdown::Both).unwrap();
    drop(clt_reader); // this will allow svc.join to complete
                      // drop(_clt_writer); // TODO critical github hangs unless write dropped, finish pod ubuntu testing with tshark and remove this drop
    let frame_send_count = svc_writer_jh.join().unwrap();
    println!(
        "frame_send_count: {:?} > frame_recv_count: {:?}, diff: {:?}",
        frame_send_count,
        frame_recv_count,
        frame_send_count - frame_recv_count,
    );

    assert!(frame_send_count > frame_recv_count);
}
