use std::{
    error::Error,
    fmt::{Debug, Display},
};

use byteserde::{prelude::from_slice, ser_stack::to_bytes_stack};
use framing::{prelude::*, Messenger};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

use super::con_frame::{FrameReader, FrameWriter};

#[derive(Debug, Clone)]
pub enum ConId {
    Clt(String),
    Svc(String),
}

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
        write!(f, "{:?}", self.con_id)
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
        msg: &mut MESSENGER::Message,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // MSGER::on_send(, msg); // TODO  complete this call back

        let (bytes, size) = to_bytes_stack::<MAX_MSG_SIZE, MESSENGER::Message>(msg)?;
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
        write!(f, "{:?}", self.con_id)
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
    ) -> Result<Option<MESSENGER::Message>, Box<dyn Error + Send + Sync>> {
        let frame = self.reader.read_frame().await?;
        if let Some(frame) = frame {
            let msg = from_slice::<MESSENGER::Message>(&frame[..])?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
}

type MessageManager<MESSENGER, const MAX_MSG_SIZE: usize, FRAMER> = (
    MessageSender<MESSENGER, MAX_MSG_SIZE>,
    MessageRecver<MESSENGER, FRAMER>,
);

pub fn into_split_messenger<MESSENGER: Messenger, const MAX_MSG_SIZE: usize, FRAMER: Framer>(
    stream: TcpStream,
    con_id: ConId,
) -> MessageManager<MESSENGER, MAX_MSG_SIZE, FRAMER> {
    let locl = format!("{}", stream.local_addr().expect("unable to get local addr"));
    let peer = format!("{}", stream.peer_addr().expect("unable to get peer addr"));
    let con_id = match con_id {
        ConId::Clt(_) => ConId::Clt(format!("{}->{}", locl, peer)),
        ConId::Svc(_) => ConId::Svc(format!("{}<-{}", locl, peer)),
    };
    match stream.into_split() {
        (reader, writer) => (
            MessageSender::new(writer, con_id.clone()),
            MessageRecver::with_capacity(reader, MAX_MSG_SIZE, con_id.clone()),
        ),
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::unittest::setup;
    use log::info;
    use soupbintcp4::prelude::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection() {
        setup::log::configure();
        let addr = setup::net::default_addr();

        const MAX_MSG_SIZE: usize = 1024;
        let svc = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let listener = TcpListener::bind(addr.clone()).await.unwrap();

                let (stream, _) = listener.accept().await.unwrap();
                let (mut sender, mut recver) = into_split_messenger::<
                    SoupBinProtocolHandler<NoPayload>,
                    MAX_MSG_SIZE,
                    SoupBinFramer,
                >(stream, ConId::Svc(addr.clone()));

                info!("{} started", recver);

                loop {
                    let msg = recver.recv().await.unwrap();
                    info!("{} RECV msg: {:?}", recver, msg);
                    match msg {
                        Some(_) => {
                            let msg =
                                &mut SoupBinMsg::<NoPayload>::dbg(b"hello world from server!");
                            sender.send(msg).await.unwrap();
                        }
                        None => {
                            info!("{} Connection Closed by Client", recver);
                            break;
                        }
                    }
                }
            })
        };
        let clt = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let stream = TcpStream::connect(addr.clone()).await.unwrap();
                let (mut sender, mut recver) = into_split_messenger::<
                    SoupBinProtocolHandler<NoPayload>,
                    MAX_MSG_SIZE,
                    SoupBinFramer,
                >(stream, ConId::Clt(addr.clone()));

                info!("{} connected", sender);
                let msg = &mut SoupBinMsg::<NoPayload>::dbg(b"hello world from client!");
                sender.send(msg).await.unwrap();
                let msg = recver.recv().await.unwrap();
                info!("{} RECV msg: {:?}", recver, msg);
            })
        };
        clt.await.unwrap();
        svc.await.unwrap();
    }
}
