use std::{
    any::type_name,
    error::Error,
    fmt::{Debug, Display},
};

use crate::prelude::*;
use byteserde::{prelude::from_slice, ser_stack::to_bytes_stack};
use log::warn;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

use super::framing::{FrameReader, FrameWriter};

#[derive(Debug)]
pub struct MessageSender<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> {
    con_id: ConId,
    writer: FrameWriter,
    phantom: std::marker::PhantomData<MESSENGER>,
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> Display
    for MessageSender<MESSENGER, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<MESSENGER>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(
            f,
            "{:?} MessageSender<{}, {}>",
            self.con_id, name, MAX_MSG_SIZE
        )
    }
}
impl<MESSENGER: Messenger, const MAX_MSG_SIZE: usize> MessageSender<MESSENGER, MAX_MSG_SIZE> {
    pub fn new(writer: OwnedWriteHalf, con_id: ConId) -> Self {
        Self {
            con_id,
            writer: FrameWriter::new(writer),
            phantom: std::marker::PhantomData,
        }
    }
    pub async fn send(
        &mut self,
        msg: &MESSENGER::SendMsg,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (bytes, size) = to_bytes_stack::<MAX_MSG_SIZE, MESSENGER::SendMsg>(msg)?;
        self.writer.write_frame(&bytes[..size]).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MessageRecver<MESSENGER: Messenger, FRAMER: Framer> {
    con_id: ConId,
    reader: FrameReader<FRAMER>,
    phantom: std::marker::PhantomData<MESSENGER>,
}
impl<MESSENGER: Messenger, FRAMER: Framer> Display for MessageRecver<MESSENGER, FRAMER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = type_name::<MESSENGER>()
            .split("::")
            .last()
            .unwrap_or("Unknown");
        write!(f, "{:?} MessageRecver<{}>", self.con_id, name)
    }
}

impl<MESSENGER: Messenger, FRAMER: Framer> MessageRecver<MESSENGER, FRAMER> {
    pub fn with_capacity(reader: OwnedReadHalf, capacity: usize, con_id: ConId) -> Self {
        Self {
            con_id,
            reader: FrameReader::with_capacity(reader, capacity),
            phantom: std::marker::PhantomData,
        }
    }
    pub async fn recv(
        &mut self,
    ) -> Result<Option<MESSENGER::RecvMsg>, Box<dyn Error + Send + Sync>> {
        let res = self.reader.read_frame().await;
        let opt_frame = match res {
            Ok(opt) => opt,
            Err(err) => {
                warn!("{} recv error: {}", self, err);
                return Err(err);
            }
        };
        if let Some(frame) = opt_frame {
            let msg = from_slice::<MESSENGER::RecvMsg>(&frame[..])?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

#[rustfmt::skip]
type MessageManager<MESSENGER, const MAX_MSG_SIZE: usize, FRAMER> = (MessageSender<MESSENGER, MAX_MSG_SIZE>, MessageRecver<MESSENGER, FRAMER>);

pub fn into_split_messenger<MESSENGER: Messenger, const MAX_MSG_SIZE: usize, FRAMER: Framer>(
    stream: TcpStream,
    con_id: ConId,
) -> MessageManager<MESSENGER, MAX_MSG_SIZE, FRAMER> {
    let (reader, writer) = stream.into_split();
    (
        MessageSender::new(writer, con_id.clone()),
        MessageRecver::with_capacity(reader, MAX_MSG_SIZE, con_id),
    )
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::unittest::setup::{model::*, protocol::*};
    use links_testing::unittest::setup;
    use log::info;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        let addr = setup::net::default_addr();

        const MAX_MSG_SIZE: usize = 1024;
        let inp_svc_msg = SvcMsg::Dbg(SvcDebugMsg::new(b"Hello Frm Server Msg"));
        let svc = {
            let addr = addr.clone();
            tokio::spawn({
                let inp_svc_msg = inp_svc_msg.clone();
                async move {
                    let listener = TcpListener::bind(addr.clone()).await.unwrap();

                    let (stream, _) = listener.accept().await.unwrap();
                    let (mut sender, mut recver) =
                        into_split_messenger::<SvcMsgProtocol, MAX_MSG_SIZE, SvcMsgProtocol>(
                            stream,
                            ConId::svc(Some("unittest"), &addr, None),
                        );
                    info!("{} connected", sender);
                    let mut out_svc_msg: Option<CltMsg> = None;
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
        let inp_clt_msg = CltMsg::Dbg(CltDebugMsg::new(b"Hello Frm Client Msg"));
        let clt = {
            let addr = addr.clone();
            tokio::spawn({
                let inp_clt_msg = inp_clt_msg.clone();
                async move {
                    let stream = TcpStream::connect(addr.clone()).await.unwrap();
                    let (mut sender, mut recver) =
                        into_split_messenger::<CltMsgProtocol, MAX_MSG_SIZE, CltMsgProtocol>(
                            stream,
                            ConId::clt(Some("unittest"), None, &addr),
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

    // TODO move to soupbin
    // #[tokio::test]
    // async fn test_connection() {
    //     setup::log::configure();
    //     let addr = setup::net::default_addr();

    //     const MAX_MSG_SIZE: usize = 1024;
    //     let svc = {
    //         let addr = addr.clone();
    //         tokio::spawn(async move {
    //             let listener = TcpListener::bind(addr.clone()).await.unwrap();

    //             let (stream, _) = listener.accept().await.unwrap();
    //             let (mut sender, mut recver) =
    //                 into_split_messenger::<
    //                     SoupBinProtocolHandler<NoPayload>,
    //                     MAX_MSG_SIZE,
    //                     SoupBinFramer,
    //                 >(stream, ConId::svc(None, &addr, None));

    //             info!("{} started", recver);

    //             loop {
    //                 let msg = recver.recv().await.unwrap();
    //                 info!("{} RECV msg: {:?}", recver, msg);
    //                 match msg {
    //                     Some(_) => {
    //                         let msg =
    //                             &mut SoupBinMsg::<NoPayload>::dbg(b"hello world from server!");
    //                         sender.send(msg).await.unwrap();
    //                     }
    //                     None => {
    //                         info!("{} Connection Closed by Client", recver);
    //                         break;
    //                     }
    //                 }
    //             }
    //         })
    //     };
    //     let clt = {
    //         let addr = addr.clone();
    //         tokio::spawn(async move {
    //             let stream = TcpStream::connect(addr.clone()).await.unwrap();
    //             let (mut sender, mut recver) =
    //                 into_split_messenger::<
    //                     SoupBinProtocolHandler<NoPayload>,
    //                     MAX_MSG_SIZE,
    //                     SoupBinFramer,
    //                 >(stream, ConId::clt(None, None, &addr));

    //             info!("{} connected", sender);
    //             let msg = &mut SoupBinMsg::<NoPayload>::dbg(b"hello world from client!");
    //             sender.send(msg).await.unwrap();
    //             let msg = recver.recv().await.unwrap();
    //             info!("{} RECV msg: {:?}", recver, msg);
    //         })
    //     };
    //     clt.await.unwrap();
    //     svc.await.unwrap();
    // }
}
