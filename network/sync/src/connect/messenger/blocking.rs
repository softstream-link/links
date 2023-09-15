use std::{any::type_name, fmt::Display, io::Error, net::TcpStream};

use links_network_core::prelude::{ConId, Messenger};

use crate::prelude_blocking::{FrameReader, FrameWriter, RecvMsg};

#[derive(Debug)]
pub struct MessageSender<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageSender<M, MAX_MSG_SIZE> {
    pub fn new(stream: TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            writer: FrameWriter::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
    #[inline(always)]
    pub fn send(&mut self, msg: &M::SendT) -> Result<(), Error> {
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        self.writer.write_frame(&bytes[..size])?;
        Ok(())
    }
}

impl<M: Messenger, const MAX_MSG_SIZE: usize> Display for MessageSender<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{:?} MessageSender<{}, {}>",
            self.con_id, name, MAX_MSG_SIZE
        )
    }
}
#[derive(Debug)]
pub struct MessageRecver<M: Messenger, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    reader: FrameReader<M, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> MessageRecver<M, MAX_MSG_SIZE> {
    pub fn new(stream: TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            reader: FrameReader::<M, MAX_MSG_SIZE>::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: Messenger, const MAX_MSG_SIZE: usize> RecvMsg<M> for MessageRecver<M, MAX_MSG_SIZE> {
    #[inline]
    fn recv(&mut self) -> Result<Option<M::RecvT>, Error> {
        let opt_bytes = self.reader.read_frame()?;
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
        write!(
            f,
            "{:?} MessageRecver<{}, {}>",
            self.con_id, name, MAX_MSG_SIZE
        )
    }
}

pub type MessageProcessor<M, const MAX_MSG_SIZE: usize> = (
    MessageRecver<M, MAX_MSG_SIZE>,
    MessageSender<M, MAX_MSG_SIZE>,
);

pub fn into_split_messenger<M: Messenger, const MAX_MSG_SIZE: usize>(
    stream: TcpStream,
    mut con_id: ConId,
) -> MessageProcessor<M, MAX_MSG_SIZE> {
    con_id.set_local(stream.local_addr().unwrap());
    con_id.set_peer(stream.peer_addr().unwrap());
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for MessageRecver"),
        stream,
    );
    (
        MessageRecver::<M, MAX_MSG_SIZE>::new(reader, con_id.clone()),
        MessageSender::<M, MAX_MSG_SIZE>::new(writer, con_id.clone()),
    )
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {

    use std::{
        net::{TcpListener, TcpStream},
        thread::{sleep, Builder},
        time::{Duration, Instant},
    };

    use crate::prelude_blocking::*;
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol};

    use links_network_core::prelude::ConId;
    use links_testing::unittest::setup::{
        self,
        model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug, TEST_MSG_FRAME_SIZE},
    };
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
                let (mut msg_sent, mut msg_recv) = (0_usize, 0_usize);
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        stream,
                        ConId::svc(Some("unittest"), addr, None),
                    );
                info!("{} connected", sender);

                while let Some(_) = recver.recv().unwrap() {
                    msg_recv += 1;
                    sender.send(&inp_svc_msg).unwrap();
                    msg_sent += 1;
                }
                info!("{} Connection Closed by Client", recver);

                (msg_sent, msg_recv)
            })
            .unwrap();

        sleep(Duration::from_millis(100)); // allow the spawned to bind

        let clt = Builder::new()
            .name("Thread-Clt".to_owned())
            .spawn(move || {
                let inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
                let (mut msg_sent_count, mut msg_recv_count) = (0, 0);
                let stream = TcpStream::connect(addr).unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        stream,
                        ConId::clt(Some("unittest"), None, addr),
                    );
                info!("{} connected", sender);
                let start = Instant::now();
                for _ in 0..WRITE_N_TIMES {
                    sender.send(&inp_clt_msg).unwrap();
                    msg_sent_count += 1;
                    let _x = recver.recv().unwrap().unwrap();
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

        assert_eq!(clt_msg_sent_count, svc_msg_sent_count);
        assert_eq!(clt_msg_recv_count, svc_msg_recv_count);
        assert_eq!(clt_msg_sent_count, WRITE_N_TIMES);
    }
}
