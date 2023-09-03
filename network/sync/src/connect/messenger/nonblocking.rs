use std::{any::type_name, error::Error, fmt::Display};

use links_network_core::prelude::{ConId, MessengerNew};

use crate::connect::framer::nonblocking::{FrameReader, FrameWriter, ReadStatus, WriteStatus};

pub struct MessageSender<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> {
    con_id: ConId,
    writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> MessageSender<M, MAX_MESSAGE_SIZE> {
    pub fn new(stream: mio::net::TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            writer: FrameWriter::new(stream),
            phantom: std::marker::PhantomData,
        }
    }

    /// Will Serialize the message using [MessengerNew] and send it over the wire as a single frame
    /// If the underlying socket is not ready to write, it will return [WriteStatus::NotReady] while
    /// also guaranteeing that non of the serialized bytes where sent. The user shall try again later in that case.
    #[inline]
    pub fn send(&mut self, msg: &M::SendT) -> Result<WriteStatus, Box<dyn Error>> {
        // TODO allow send of a frame so that the retry does not incur the cost of serialization
        let (bytes, size) = M::serialize::<MAX_MESSAGE_SIZE>(msg)?;
        self.writer.write_frame(&bytes[..size])
    }
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> Display
    for MessageSender<M, MAX_MESSAGE_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{:?} MessageSender<{}, {}>",
            self.con_id, name, MAX_MESSAGE_SIZE
        )
    }
}

pub struct MessageRecver<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> {
    con_id: ConId,
    reader: FrameReader<M, MAX_MESSAGE_SIZE>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> MessageRecver<M, MAX_MESSAGE_SIZE> {
    pub fn new(stream: mio::net::TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            reader: FrameReader::<M, MAX_MESSAGE_SIZE>::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
    #[inline]
    pub fn recv(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>> {
        let status = self.reader.read_frame()?;
        match status {
            ReadStatus::Completed(Some(frame)) => {
                let msg = M::deserialize(&frame)?;
                Ok(ReadStatus::Completed(Some(msg)))
            }
            ReadStatus::Completed(None) => Ok(ReadStatus::Completed(None)),
            ReadStatus::NotReady => Ok(ReadStatus::NotReady),
        }
    }
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> Display
    for MessageRecver<M, MAX_MESSAGE_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{:?} MessageRecver<{}, {}>",
            self.con_id, name, MAX_MESSAGE_SIZE
        )
    }
}

pub type MessageProcessor<M, const MAX_MESSAGE_SIZE: usize> = (
    MessageRecver<M, MAX_MESSAGE_SIZE>,
    MessageSender<M, MAX_MESSAGE_SIZE>,
);

pub fn into_split_messenger<M, const MAX_MESSAGE_SIZE: usize>(
    stream: std::net::TcpStream,
    con_id: ConId,
) -> MessageProcessor<M, MAX_MESSAGE_SIZE>
where
    M: MessengerNew,
{
    stream
        .set_nonblocking(true)
        .expect("Failed to set nonblocking on TcpStream");
    // TODO this causes performance to go from 800ns to 2.5µs
    // stream
    //     .set_nodelay(true)
    //     .expect("Failed to set_nodelay on TcpStream");
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for MessageRecver"),
        stream,
    );
    (
        MessageRecver::<M, MAX_MESSAGE_SIZE>::new(
            mio::net::TcpStream::from_std(reader),
            con_id.clone(),
        ),
        MessageSender::<M, MAX_MESSAGE_SIZE>::new(mio::net::TcpStream::from_std(writer), con_id),
    )
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use std::{
        thread::{sleep, Builder},
        time::{Duration, Instant},
    };

    use links_network_core::prelude::ConId;
    use links_testing::unittest::setup::{self, model::*};
    use log::info;
    use num_format::{Locale, ToFormattedString};

    use crate::{
        connect::{
            framer::nonblocking::{ReadStatus, WriteStatus},
            messenger::nonblocking::into_split_messenger,
        },
        unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol},
    };

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
                        stream,
                        ConId::svc(Some("unittest"), addr, None),
                    );
                info!("{} connected", sender);

                while let Ok(status) = recver.recv() {
                    match status {
                        ReadStatus::Completed(Some(_recv_msg)) => {
                            msg_recv_count += 1;
                            while let WriteStatus::NotReady = sender.send(&inp_svc_msg).unwrap() {}
                            msg_sent_count += 1;
                        }
                        ReadStatus::Completed(None) => {
                            info!("{} Connection Closed by Client", recver);
                            break;
                        }
                        ReadStatus::NotReady => continue,
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
                        stream,
                        ConId::clt(Some("unittest"), None, addr),
                    );

                let start = Instant::now();
                for _ in 0..WRITE_N_TIMES {
                    while let WriteStatus::NotReady = sender.send(&inp_clt_msg).unwrap() {}
                    msg_sent_count += 1;
                    while let ReadStatus::NotReady = recver.recv().unwrap() {}
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
