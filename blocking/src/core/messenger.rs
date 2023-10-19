//! This module contains a blocking `paired` [MessageRecver] and [MessageSender] which are designed to be used in different threads,
//! where each thread is only doing either send or recv to the underlying [std::net::TcpStream] via respective [FrameReader] and [FrameWriter].
//!
//! # Note
//! The underlying [std::net::TcpStream] is cloned and therefore share a single underlying network socket.
//!
//! # Example
//! ```
//! use links_blocking::prelude::*;
//! use links_core::unittest::setup::framer::{CltTestMessenger, SvcTestMessenger, TEST_MSG_FRAME_SIZE};
//!
//! let addr = "127.0.0.1:8080";
//!
//! let svc_listener = std::net::TcpListener::bind(addr).unwrap();
//!
//! let clt_stream = std::net::TcpStream::connect(addr).unwrap();
//! let (clt_recv, clt_send) = into_split_messenger::<CltTestMessenger, TEST_MSG_FRAME_SIZE>(
//!         ConId::clt(Some("unittest"), None, addr),
//!         clt_stream,
//!     );
//!
//! let svc_stream = svc_listener.accept().unwrap().0;
//! let (svc_recv, svc_send) = into_split_messenger::<SvcTestMessenger, TEST_MSG_FRAME_SIZE>(
//!         ConId::svc(Some("unittest"), addr, None),
//!         svc_stream,
//!     );
//!
//! drop(clt_recv);
//! drop(clt_send);
//! drop(svc_recv);
//! drop(svc_send);
//! drop(svc_listener);
//!
//! // Note:
//!     // paired
//!         // clt_recv & clt_send
//!         // svc_recv & svc_send
//!     // peers
//!         // clt_recv & svc_send
//!         // svc_recv & clt_send
//! ```

use std::{any::type_name, fmt::Display, io::Error, net::TcpStream};

use links_core::prelude::{ConId, Messenger};

use crate::prelude::{FrameReader, FrameWriter, RecvMsg, SendMsgNonMut};

// Represents an abstraction for receiving exactly one message utilizing the underlying [FrameReader].
#[derive(Debug)]
pub struct MessageRecver<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) frm_reader: FrameReader<M, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageRecver<M, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, stream: TcpStream) -> Self {
        Self {
            frm_reader: FrameReader::<M, MAX_MSG_SIZE>::new(con_id, stream),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> RecvMsg<M> for MessageRecver<M, MAX_MSG_SIZE> {
    #[inline(always)]
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error> {
        let opt_bytes = self.frm_reader.read_frame()?;
        match opt_bytes {
            Some(frame) => {
                let msg = M::deserialize(&frame)?;
                Ok(Some(msg))
            }
            None => Ok(None),
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> Display for MessageRecver<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "{:?} MessageRecver<{}, {}>", self.frm_reader.con_id, name, MAX_MSG_SIZE)
    }
}

/// Represents an abstraction for sending exactly one message utilizing the underlying [FrameWriter]
#[derive(Debug)]
pub struct MessageSender<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) frm_writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageSender<M, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, stream: TcpStream) -> Self {
        Self {
            frm_writer: FrameWriter::new(con_id, stream),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> SendMsgNonMut<M> for MessageSender<M, MAX_MSG_SIZE> {
    #[inline(always)]
    fn send(&mut self, msg: &<M as Messenger>::SendT) -> Result<(), Error> {
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        self.frm_writer.write_frame(&bytes[..size])?;
        Ok(())
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> Display for MessageSender<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let messenger_name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "{} MessageSender<{}, {}>", self.frm_writer.con_id, messenger_name, MAX_MSG_SIZE)
    }
}

pub type MessageProcessor<M, const MAX_MSG_SIZE: usize> = (MessageRecver<M, MAX_MSG_SIZE>, MessageSender<M, MAX_MSG_SIZE>);

/// Creates a `paired` [MessageRecver] and [MessageSender] from a [std::net::TcpStream] by cloning it.
///
/// # Returns a tuple with
///  * [MessageRecver] - for receiving messages
///  * [MessageSender] - for sending messages
///
/// # Important
/// if either [MessageRecver] or [MessageSender] is dropped, the underlying stream will be shutdown and all actions on the remaining `pair` will fail
pub fn into_split_messenger<M: Messenger, const MAX_MSG_SIZE: usize>(mut con_id: ConId, stream: TcpStream) -> MessageProcessor<M, MAX_MSG_SIZE> {
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
    let (reader, writer) = (stream.try_clone().expect("Failed to try_clone TcpStream for MessageRecver"), stream);
    (MessageRecver::<M, MAX_MSG_SIZE>::new(con_id.clone(), reader), MessageSender::<M, MAX_MSG_SIZE>::new(con_id, writer))
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use std::{
        io::ErrorKind,
        net::{TcpListener, TcpStream},
        thread::{sleep, Builder},
        time::{Duration, Instant},
    };

    use crate::prelude::*;

    use links_core::{
        fmt_num,
        prelude::ConId,
        unittest::setup::{
            self,
            framer::{CltTestMessenger, SvcTestMessenger},
            model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug, TEST_MSG_FRAME_SIZE},
        },
    };
    use log::info;
    use rand::Rng;

    #[test]
    fn test_messenger() {
        setup::log::configure_level(log::LevelFilter::Info);
        let addr = setup::net::rand_avail_addr_port();

        const WRITE_N_TIMES: usize = 100_000;

        let svc = Builder::new()
            .name("Thread-Svc".to_owned())
            .spawn(move || {
                let inp_svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
                let (mut svc_msg_sent_count, mut svc_msg_recv_count) = (0_usize, 0_usize);
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut svc_recver, mut svc_sender) = into_split_messenger::<SvcTestMessenger, TEST_MSG_FRAME_SIZE>(ConId::svc(Some("unittest"), addr, None), stream);
                info!("{} connected", svc_sender);

                while let Some(_) = svc_recver.recv().unwrap() {
                    svc_msg_recv_count += 1;
                    svc_sender.send(&inp_svc_msg).unwrap();
                    svc_msg_sent_count += 1;
                }
                info!("{} Connection Closed by Client", svc_recver);

                (svc_msg_sent_count, svc_msg_recv_count)
            })
            .unwrap();

        sleep(Duration::from_millis(100)); // allow the spawned to bind

        let inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let (mut clt_msg_sent_count, mut clt_msg_recv_count) = (0, 0);
        let stream = TcpStream::connect(addr).unwrap();
        let (mut clt_recver, mut clt_sender) = into_split_messenger::<CltTestMessenger, TEST_MSG_FRAME_SIZE>(ConId::clt(Some("unittest"), None, addr), stream);
        info!("{} connected", clt_sender);
        let start = Instant::now();
        for _ in 0..WRITE_N_TIMES {
            clt_sender.send(&inp_clt_msg).unwrap();
            clt_msg_sent_count += 1;
            let _x = clt_recver.recv().unwrap().unwrap();
            clt_msg_recv_count += 1;
        }
        let elapsed = start.elapsed();

        if rand::thread_rng().gen_range(1..=2) % 2 == 0 {
            info!("dropping clt_sender");
            drop(clt_sender);
            let opt = clt_recver.recv().unwrap();
            info!("clt_recver.recv(): {:?}", opt);
            assert_eq!(opt, None);
        } else {
            info!("dropping clt_recver");
            drop(clt_recver);
            let err = clt_sender.send(&inp_clt_msg).unwrap_err();
            info!("clt_sender.send(): {}", err);
            assert_eq!(err.kind(), ErrorKind::BrokenPipe);
        }

        let (svc_msg_sent_count, svc_msg_recv_count) = svc.join().unwrap();
        info!("clt_msg_sent_count: {}, clt_msg_recv_count: {}", fmt_num!(clt_msg_sent_count), fmt_num!(clt_msg_recv_count));
        info!("svc_msg_sent_count: {}, svc_msg_recv_count: {}", fmt_num!(svc_msg_sent_count), fmt_num!(svc_msg_recv_count));
        info!("per round trip elapsed: {:?}, total elapsed: {:?} ", elapsed / WRITE_N_TIMES as u32, elapsed);

        assert_eq!(clt_msg_sent_count, svc_msg_sent_count);
        assert_eq!(clt_msg_recv_count, svc_msg_recv_count);
        assert_eq!(clt_msg_sent_count, WRITE_N_TIMES);
    }
}
