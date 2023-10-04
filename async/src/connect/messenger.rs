use std::{
    any::type_name,
    error::Error,
    fmt::{Debug, Display},
    sync::Arc,
};

use byteserde::prelude::{from_slice, to_bytes_stack};
use links_core::prelude::{ConId, Framer, MessengerOld};
use log::warn;
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

use super::framer::{FrameReader, FrameWriter};

pub type MsgRecverRef<P, F> = Arc<Mutex<MessageRecver<P, F>>>;
pub type MsgSenderRef<P, const MMS: usize> = Arc<Mutex<MessageSender<P, MMS>>>;

#[derive(Debug)]
pub struct MessageSender<M: MessengerOld, const MMS: usize> {
    con_id: ConId,
    writer: FrameWriter,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerOld, const MMS: usize> MessageSender<M, MMS> {
    pub fn new(writer: OwnedWriteHalf, con_id: ConId) -> Self {
        Self {
            con_id,
            writer: FrameWriter::new(writer),
            phantom: std::marker::PhantomData,
        }
    }
    pub async fn send(&mut self, msg: &M::SendT) -> Result<(), Box<dyn Error+Send+Sync>> {
        let (bytes, size) = to_bytes_stack::<MMS, M::SendT>(msg)?;
        self.writer.write_frame(&bytes[..size]).await?;
        Ok(())
    }
}
impl<M: MessengerOld, const MMS: usize> Display for MessageSender<M, MMS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "{:?} MessageSender<{}, {}>", self.con_id, name, MMS)
    }
}

#[derive(Debug)]
pub struct MessageRecver<M: MessengerOld, F: Framer> {
    con_id: ConId,
    reader: FrameReader<F>,
    phantom: std::marker::PhantomData<M>,
}
impl<M: MessengerOld, F: Framer> MessageRecver<M, F> {
    pub fn with_max_frame_size(
        reader: OwnedReadHalf,
        reader_max_frame_size: usize,
        con_id: ConId,
    ) -> Self {
        Self {
            con_id,
            reader: FrameReader::with_max_frame_size(reader, reader_max_frame_size),
            phantom: std::marker::PhantomData,
        }
    }
    pub async fn recv(&mut self) -> Result<Option<M::RecvT>, Box<dyn Error+Send+Sync>> {
        let res = self.reader.read_frame().await;
        let opt_frame = match res {
            Ok(opt) => opt,
            Err(err) => {
                warn!("{} recv error: {}", self, err);
                return Err(err);
            }
        };
        if let Some(frame) = opt_frame {
            let msg = from_slice::<M::RecvT>(&frame[..])?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}
impl<M: MessengerOld, F: Framer> Display for MessageRecver<M, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<M>().split("::").last().unwrap_or("Unknown");
        write!(f, "{:?} MessageRecver<{}>", self.con_id, name)
    }
}

#[rustfmt::skip]
type MessageProcessor<M, const MMS: usize, F> = (MessageSender<M, MMS>, MessageRecver<M, F>);

pub fn into_split_messenger<M: MessengerOld, const MMS: usize, F: Framer>(
    stream: TcpStream,
    con_id: ConId,
) -> MessageProcessor<M, MMS, F> {
    let (reader, writer) = stream.into_split();
    (
        MessageSender::new(writer, con_id.clone()),
        MessageRecver::with_max_frame_size(reader, MMS, con_id),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::unittest::setup::protocol::*;
    use links_core::unittest::setup::{self, model::*};
    use log::info;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_messenger() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        const MMS: usize = 1024;
        let inp_svc_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg"));
        let svc = {
            tokio::spawn({
                let inp_svc_msg = inp_svc_msg.clone();
                async move {
                    let listener = TcpListener::bind(addr).await.unwrap();

                    let (stream, _) = listener.accept().await.unwrap();
                    let (mut sender, mut recver) =
                        into_split_messenger::<TestSvcMsgProtocol, MMS, TestSvcMsgProtocol>(
                            stream,
                            ConId::svc(Some("unittest"), addr, None),
                        );
                    info!("{} connected", sender);
                    let mut out_svc_msg: Option<TestCltMsg> = None;
                    loop {
                        let opt = recver.recv().await.unwrap();
                        match opt {
                            Some(msg) => {
                                out_svc_msg = Some(msg);
                                sender.send(&inp_svc_msg).await.unwrap();
                            }
                            None => {
                                info!("{} Connection Closed by Client", recver);
                                break;
                            }
                        }
                    }
                    out_svc_msg.unwrap()
                }
            })
        };
        let inp_clt_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        let clt = {
            tokio::spawn({
                let inp_clt_msg = inp_clt_msg.clone();
                async move {
                    let stream = TcpStream::connect(addr).await.unwrap();
                    let (mut sender, mut recver) =
                        into_split_messenger::<TestCltMsgProtocol, MMS, TestCltMsgProtocol>(
                            stream,
                            ConId::clt(Some("unittest"), None, addr),
                        );
                    info!("{} connected", sender);
                    sender.send(&inp_clt_msg).await.unwrap();
                    let out_clt_msg = recver.recv().await.unwrap();
                    out_clt_msg.unwrap()
                }
            })
        };
        let out_clt_msg = clt.await.unwrap();
        let out_svc_msg = svc.await.unwrap();
        info!("inp_clt_msg: {:?}", inp_clt_msg);
        info!("out_clt_msg: {:?}", out_clt_msg);
        info!("inp_svc_msg: {:?}", inp_svc_msg);
        info!("out_svc_msg: {:?}", out_svc_msg);
        assert_eq!(inp_clt_msg, out_svc_msg);
        assert_eq!(inp_svc_msg, out_clt_msg);
    }
}
