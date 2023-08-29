use std::{any::type_name, error::Error, fmt::Display, net::TcpStream};

use links_network_core::{core::MessengerNew, prelude::ConId};

use crate::connect::framer::{blocking::FrameReader, blocking::FrameWriter};

pub struct MessageSender<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> {
    con_id: ConId,
    writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> MessageSender<M, MAX_MESSAGE_SIZE> {
    pub fn new(stream: TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            writer: FrameWriter::new(stream),
            phantom: std::marker::PhantomData,
        }
    }
    pub fn send(&mut self, msg: &M::SendT) -> Result<(), Box<dyn Error>> {
        let (bytes, size) = M::serialize::<MAX_MESSAGE_SIZE>(msg)?;
        self.writer.write_frame(&bytes[..size])?;
        Ok(())
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
    reader: FrameReader<M>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> MessageRecver<M, MAX_MESSAGE_SIZE> {
    pub fn new(stream: TcpStream, con_id: ConId) -> Self {
        Self {
            con_id,
            reader: FrameReader::<M>::with_max_frame_size(stream, MAX_MESSAGE_SIZE), // TODO how to inject
            phantom: std::marker::PhantomData,
        }
    }
    pub fn recv(&mut self) -> Result<Option<M::RecvT>, Box<dyn Error>> {
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

type MessageProcessor<M, const MAX_MESSAGE_SIZE: usize> = (
    MessageRecver<M, MAX_MESSAGE_SIZE>,
    MessageSender<M, MAX_MESSAGE_SIZE>,
);

pub fn into_split_messenger<M, const MAX_MESSAGE_SIZE: usize>(
    stream: TcpStream,
    con_id: ConId,
) -> MessageProcessor<M, MAX_MESSAGE_SIZE>
where
    M: MessengerNew,
{
    let (reader, writer) = (
        stream
            .try_clone()
            .expect("Failed to try_clone TcpStream for MessageRecver"),
        stream,
    );
    (
        MessageRecver::<M, MAX_MESSAGE_SIZE>::new(reader, con_id.clone()),
        MessageSender::<M, MAX_MESSAGE_SIZE>::new(writer, con_id),
    )
}

#[cfg(test)]
#[cfg(feature = "unittest")]
mod test {
    use crate::unittest::setup::framer::{TestCltMsgProtocol, TestSvcMsgProtocol};
    use std::{net::TcpListener, thread::Builder};

    use super::*;
    use links_testing::unittest::setup::{
        self,
        model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug, TEST_MSG_FRAME_SIZE},
    };
    use log::info;

    #[test]
    fn test_messenger() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let inp_svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
        let svc = Builder::new().name("Thread-Svc".to_owned()).spawn({
            let inp_svc_msg = inp_svc_msg.clone();
            move || {
                let listener = TcpListener::bind(addr).unwrap();
                let (stream, _) = listener.accept().unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        stream,
                        ConId::svc(Some("unittest"), addr, None),
                    );
                info!("{} connected", sender);
                let mut out_svc_msg: Option<TestCltMsg> = None;
                loop {
                    let opt = recver.recv().unwrap();
                    match opt {
                        Some(msg) => {
                            out_svc_msg = Some(msg);
                            sender.send(&inp_svc_msg).unwrap();
                        }
                        None => {
                            info!("{} Connection Closed by Client", recver);
                            break;
                        }
                    }
                }
                out_svc_msg.unwrap()
            }
        }).unwrap();

        let inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let clt = Builder::new().name("Thread-Clt".to_owned()).spawn({
            let inp_clt_msg = inp_clt_msg.clone();
            move || {
                let stream = TcpStream::connect(addr).unwrap();
                let (mut recver, mut sender) =
                    into_split_messenger::<TestCltMsgProtocol, TEST_MSG_FRAME_SIZE>(
                        stream,
                        ConId::clt(Some("unittest"), None, addr),
                    );
                info!("{} connected", sender);
                sender.send(&inp_clt_msg).unwrap();
                let out_clt_msg = recver.recv().unwrap();
                out_clt_msg.unwrap()
            }
        }).unwrap();

        let out_clt_msg = clt.join().unwrap();
        let out_svc_msg = svc.join().unwrap();
        info!("inp_clt_msg: {:?}", inp_clt_msg);
        info!("out_clt_msg: {:?}", out_clt_msg);
        info!("inp_svc_msg: {:?}", inp_svc_msg);
        info!("out_svc_msg: {:?}", out_svc_msg);
        assert_eq!(inp_clt_msg, out_svc_msg);
        assert_eq!(inp_svc_msg, out_clt_msg);
    }
}
