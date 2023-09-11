use std::{any::type_name, error::Error, fmt::Display};

use crate::connect::framer::nonblocking::{FrameReader, FrameWriter};
use crate::prelude_nonblocking::*;
use links_network_core::prelude::{ConId, MessengerNew};

#[derive(Debug)]
pub struct MessageSender<M: MessengerNew, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    frm_writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> MessageSender<M, MAX_MSG_SIZE> {
    pub fn new(stream: mio::net::TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            frm_writer: FrameWriter::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
}
#[rustfmt::skip]
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> SendMsgNonBlocking<M> for MessageSender<M, MAX_MSG_SIZE>{
    /// Will Serialize the message using [MessengerNew] and send it over the wire as a single frame
    /// If the underlying socket is not ready to write, it will return [WriteStatus::NotReady] while
    /// also guaranteeing that non of the serialized bytes where sent. The user shall try again later in that case.
    #[inline(always)]
    fn send_nonblocking(&mut self, msg: &M::SendT) -> Result<WriteStatus, Box<dyn Error>> {
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        self.frm_writer.write_frame(&bytes[..size])
    }
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> SendMsgBusyWaitMut<M>
    for MessageSender<M, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn send_busywait(
        &mut self,
        msg: &mut <M as MessengerNew>::SendT,
    ) -> Result<(), Box<dyn Error>> {
        let (bytes, size) = M::serialize::<MAX_MSG_SIZE>(msg)?;
        while let WriteStatus::WouldBlock = self.frm_writer.write_frame(&bytes[..size])? {
            // busy wait tuntill write_frame returns WriteStatus::Completed
            continue;
        }
        Ok(())
    }
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> Display for MessageSender<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msger = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{} MessageSender<{}, {}>",
            self.con_id, msger, MAX_MSG_SIZE
        )
    }
}

#[derive(Debug)]
pub struct MessageRecver<M: MessengerNew, const MAX_MSG_SIZE: usize> {
    pub(crate) con_id: ConId,
    frm_reader: FrameReader<M, MAX_MSG_SIZE>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> MessageRecver<M, MAX_MSG_SIZE> {
    pub fn new(stream: mio::net::TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            frm_reader: FrameReader::<M, MAX_MSG_SIZE>::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> RecvMsgNonBlocking<M>
    for MessageRecver<M, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_nonblocking(&mut self) -> Result<ReadStatus<M::RecvT>, Box<dyn Error>> {
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
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> RecvMsgBusyWait<M>
    for MessageRecver<M, MAX_MSG_SIZE>
{
    #[inline(always)]
    fn recv_busywait(&mut self) -> Result<Option<<M as MessengerNew>::RecvT>, Box<dyn Error>> {
        loop {
            match self.recv_nonblocking()? {
                ReadStatus::Completed(opt) => return Ok(opt),
                ReadStatus::WouldBlock => continue,
            }
        }
    }
}
impl<M: MessengerNew, const MAX_MSG_SIZE: usize> Display for MessageRecver<M, MAX_MSG_SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(
            f,
            "{} MessageRecver<{}, {}>",
            self.con_id, name, MAX_MSG_SIZE
        )
    }
}

pub type MessageProcessor<M, const MAX_MSG_SIZE: usize> = (
    MessageRecver<M, MAX_MSG_SIZE>,
    MessageSender<M, MAX_MSG_SIZE>,
);

pub fn into_split_messenger<M: MessengerNew, const MAX_MSG_SIZE: usize>(
    stream: std::net::TcpStream,
    con_id: ConId,
) -> MessageProcessor<M, MAX_MSG_SIZE> {
    stream
        .set_nonblocking(true)
        .expect("Failed to set nonblocking on TcpStream");
    // TODO this causes performance to go from 800ns to 2.5µs
    // stream
    //     .set_nodelay(true)
    //     .expect("Failed to set_nodelay on TcpStream");

    let mut con_id = con_id.clone();
    con_id.set_local(stream.local_addr().unwrap());
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for MessageRecver"),
        stream,
    );

    (
        MessageRecver::<M, MAX_MSG_SIZE>::new(
            mio::net::TcpStream::from_std(reader),
            con_id.clone(),
        ),
        MessageSender::<M, MAX_MSG_SIZE>::new(
            mio::net::TcpStream::from_std(writer),
            con_id.clone(),
        ),
    )
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use std::{
        thread::{sleep, Builder},
        time::{Duration, Instant},
    };

    use crate::prelude_nonblocking::*;
    use links_network_core::prelude::ConId;
    use links_testing::unittest::setup::{self, model::*};
    use log::info;
    use num_format::{Locale, ToFormattedString};

    use crate::{
        connect::messenger::nonblocking::into_split_messenger,
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
                        stream,
                        ConId::clt(Some("unittest"), None, addr),
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
