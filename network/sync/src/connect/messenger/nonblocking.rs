use std::{
    any::type_name,
    fmt::Display,
    io::Error,
    time::{Duration, Instant},
};

use crate::prelude_nonblocking::{
    FrameReader, FrameWriter, ReadStatus, RecvMsgNonBlocking, WriteStatus,
};
use links_network_core::prelude::{ConId, Messenger};

#[derive(Debug)]
pub struct MessageSender<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) frm_writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageSender<M, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, stream: mio::net::TcpStream) -> Self {
        Self {
            frm_writer: FrameWriter::new(con_id, stream),
            phantom: std::marker::PhantomData,
        }
    }
    /// If there was a successfull attempt to write any bytes from serialized message
    /// into the stream but the write was only partial then the call shall buzy wait until all
    /// remaining bytes were written before returning [WriteStatus::Completed].
    /// [WriteStatus::WouldBlock] is returned only if the attemp did not write any bytes to the stream
    /// after the first attempt
    #[inline(always)]
    pub fn send_nonblocking(&mut self, msg: &M::SendT) -> Result<WriteStatus, Error> {
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        self.frm_writer.write_frame(&bytes[..size])
    }

    /// Will call [send_nonblocking] untill it returns [WriteStatus::Completed] or [WriteStatus::WouldBlock] after the timeoutok,
    #[inline(always)]
    pub fn send_nonblocking_timeout(
        &mut self,
        msg: &M::SendT,
        timeout: Duration,
    ) -> Result<WriteStatus, Error> {
        let start = Instant::now();
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        loop {
            match self.frm_writer.write_frame(&bytes[..size])? {
                WriteStatus::Completed => return Ok(WriteStatus::Completed),
                WriteStatus::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Ok(WriteStatus::WouldBlock);
                    }
                }
            }
        }
    }
    /// will busywait block on [send_nonblocking] untill it returns [WriteStatus::Completed]
    #[inline(always)]
    pub fn send_busywait(&mut self, msg: &M::SendT) -> Result<(), Error> {
        loop {
            match self.send_nonblocking(msg)? {
                WriteStatus::Completed => return Ok(()),
                WriteStatus::WouldBlock => continue,
            }
        }
    }
}

impl<M: Messenger, const MAX_MSG_SIZE: usize> Display for MessageSender<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msger = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{} MessageSender<{}, {}>",
            self.frm_writer.con_id, msger, MAX_MSG_SIZE
        )
    }
}

#[derive(Debug)]
pub struct MessageRecver<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) frm_reader: FrameReader<M, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageRecver<M, MAX_MSG_SIZE> {
    pub fn new(con_id: ConId, stream: mio::net::TcpStream) -> Self {
        Self {
            frm_reader: FrameReader::<M, MAX_MSG_SIZE>::new(con_id, stream),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for MessageRecver<M, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Error> {
        let status = self.frm_reader.read_frame()?;
        match status {
            ReadStatus::Completed(Some(frame)) => {
                let msg = M::deserialize(&frame)?;
                Ok(ReadStatus::Completed(Some(msg)))
            }
            ReadStatus::Completed(None) => Ok(ReadStatus::Completed(None)),
            ReadStatus::WouldBlock => Ok(ReadStatus::WouldBlock),
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> Display for MessageRecver<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{} MessageRecver<{}, {}>",
            self.frm_reader.con_id, name, MAX_MSG_SIZE
        )
    }
}

pub type MessageProcessor<M, const MAX_MSG_SIZE: usize> = (
    MessageRecver<M, MAX_MSG_SIZE>,
    MessageSender<M, MAX_MSG_SIZE>,
);

pub fn into_split_messenger<M: Messenger, const MAX_MSG_SIZE: usize>(
    mut con_id: ConId,
    stream: std::net::TcpStream,
) -> MessageProcessor<M, MAX_MSG_SIZE> {
    stream
        .set_nonblocking(true)
        .expect("Failed to set nonblocking on TcpStream");
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for MessageRecver"),
        stream,
    );

    let (reader, writer) = (
        mio::net::TcpStream::from_std(reader),
        mio::net::TcpStream::from_std(writer),
    );
    (
        MessageRecver::<M, MAX_MSG_SIZE>::new(con_id.clone(), reader),
        MessageSender::<M, MAX_MSG_SIZE>::new(con_id, writer),
    )
}

/// # Warning!!!
/// This is not a public trait as it [send_nonblocking] implementaiton requires special consideration if it involves modification of state as it is expected
/// to be called repetedly to recover from [WriteStatus::WouldBlock]

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use std::{
        thread::{sleep, Builder},
        time::{Duration, Instant},
    };

    use crate::prelude_nonblocking::*;
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol};

    use links_network_core::prelude::ConId;
    use links_testing::unittest::setup::{self, model::*};
    use log::info;
    use num_format::{Locale, ToFormattedString};

    #[test]
    fn test_messenger() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();
        const WRITE_N_TIMES: usize = 100_000;

        let svc = Builder::new()
            .name("Thread-Svc".to_owned())
            .spawn(move || {
                let inp_svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
                let (mut msg_sent_count, mut msg_recv_count) = (0_usize, 0_usize);
                let listener = std::net::TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        ConId::svc(Some("unittest"), addr, None),
                        stream,
                    );
                info!("{} connected", sender);

                while let Ok(status) = recver.recv_nonblocking() {
                    match status {
                        ReadStatus::Completed(Some(_recv_msg)) => {
                            msg_recv_count += 1;
                            while let WriteStatus::WouldBlock =
                                sender.send_nonblocking(&inp_svc_msg).unwrap()
                            {
                            }
                            msg_sent_count += 1;
                        }
                        ReadStatus::Completed(None) => {
                            info!("{} Connection Closed by Client", recver);
                            break;
                        }
                        ReadStatus::WouldBlock => continue,
                    }
                }
                (msg_sent_count, msg_recv_count)
            })
            .unwrap();

        sleep(Duration::from_millis(100)); // allow the spawned to bind

        let clt = Builder::new()
            .name("Thread-Clt".to_owned())
            .spawn(move || {
                let inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
                let (mut msg_sent_count, mut msg_recv_count) = (0, 0);
                let stream = std::net::TcpStream::connect(addr).unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        ConId::clt(Some("unittest"), None, addr),
                        stream,
                    );

                let start = Instant::now();
                for _ in 0..WRITE_N_TIMES {
                    while let WriteStatus::WouldBlock =
                        sender.send_nonblocking(&inp_clt_msg).unwrap()
                    {}
                    msg_sent_count += 1;
                    while let ReadStatus::WouldBlock = recver.recv_nonblocking().unwrap() {}
                    msg_recv_count += 1;
                }

                (msg_sent_count, msg_recv_count, start.elapsed())
            })
            .unwrap();

        let (clt_msg_sent_count, clt_msg_recv_count, elapsed) = clt.join().unwrap();
        let (svc_msg_sent_count, svc_msg_recv_count) = svc.join().unwrap();
        info!(
            "clt_msg_sent_count: {}, clt_msg_recv_count: {}",
            clt_msg_sent_count.to_formatted_string(&Locale::en),
            clt_msg_recv_count.to_formatted_string(&Locale::en)
        );
        info!(
            "svc_msg_sent_count: {}, svc_msg_recv_count: {}",
            svc_msg_sent_count.to_formatted_string(&Locale::en),
            svc_msg_recv_count.to_formatted_string(&Locale::en)
        );
        info!(
            "per round trip elapsed: {:?}, total elapsed: {:?} ",
            elapsed / WRITE_N_TIMES as u32,
            elapsed
        );
    }
}
