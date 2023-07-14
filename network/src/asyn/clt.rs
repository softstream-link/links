use std::{
    error::Error,
    fmt::Display,
    sync::Arc,
    time::{Duration, Instant},
};

use log::info;

use tokio::sync::Mutex;
use tokio::time::sleep;

use tokio::spawn;

use framing::MessageHandler;
use tokio::net::TcpStream;

use super::con_msg::{StreamMessenderReader, StreamMessenderWriter};

#[derive(Debug, Clone)]
pub enum ConId {
    Clt(String),
    Svc(String),
}

pub type CltReaderRef<HANDLER> = Arc<Mutex<StreamMessenderReader<HANDLER>>>;
pub type CltWriterRef<HANDLER> = Arc<Mutex<StreamMessenderWriter<HANDLER>>>;

#[derive(Debug)]
pub struct CltWriter<HANDLER: MessageHandler> {
    con_id: ConId,
    writer: CltWriterRef<HANDLER>,
}
impl<HANDLER: MessageHandler> Display for CltWriter<HANDLER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}
impl<HANDLER: MessageHandler> CltWriter<HANDLER> {
    pub async fn send<const MAX_MSG_SIZE: usize>(
        &mut self,
        msg: &HANDLER::Item,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut writer = self.writer.lock().await;
        writer.send::<MAX_MSG_SIZE>(msg).await
    }
}

#[derive(Debug)]
pub struct Clt<HANDLER: MessageHandler> {
    con_id: ConId,
    reader: CltReaderRef<HANDLER>,
    writer: CltWriterRef<HANDLER>,
}
impl<HANLDER: MessageHandler> Display for Clt<HANLDER> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.con_id)
    }
}

impl<HANDLER: MessageHandler> Clt<HANDLER> {
    pub async fn new(
        addr: &str,
        timeout: Duration,
        retry_after: Duration,
    ) -> Result<CltWriter<HANDLER>, Box<dyn Error + Send + Sync>> {
        assert!(timeout > retry_after);
        let now = Instant::now();
        let con_id = ConId::Clt(addr.to_owned());
        while now.elapsed() < timeout {
            let res = TcpStream::connect(addr).await;
            match res {
                Err(e) => {
                    info!("{:?} connect failed. e: {:?}", con_id, e);
                    sleep(retry_after).await;
                    continue;
                }
                Ok(stream) => {
                    let con_id = ConId::Clt(format!(
                        "{:?}->{:?}",
                        stream.local_addr()?,
                        stream.peer_addr()?
                    ));
                    info!("{:?} connected", con_id);
                    return Ok(Self::from_stream(stream, con_id).await);
                }
            }
        }
        Err(format!("{:?} connect timeout: {:?}", con_id, timeout).into())
    }
    pub async fn from_stream(stream: TcpStream, con_id: ConId) -> CltWriter<HANDLER> {
        let (read, write) = stream.into_split();
        let read_ref =
            CltReaderRef::new(Mutex::new(StreamMessenderReader::new(read, con_id.clone())));
        let write_ref = CltWriterRef::new(Mutex::new(StreamMessenderWriter::new(
            write,
            con_id.clone(),
        )));
        let clt = Self {
            con_id: con_id.clone(),
            reader: read_ref.clone(),
            writer: write_ref.clone(),
        };
        {
            let con_id = con_id.clone();
            spawn(async move {
                info!("{:?} stream started", con_id);
                let res = Self::run(clt).await;
                match res {
                    Ok(()) => {
                        info!("{:?} stream stopped", con_id);
                    }
                    Err(e) => {
                        info!("{:?} stream exit err:: {:?}", con_id, e);
                    }
                }
            });
        }

        CltWriter {
            con_id: con_id.clone(),
            writer: write_ref,
        }
    }

    async fn run(clt: Clt<HANDLER>) -> Result<(), Box<dyn Error + Sync + Send>> {
        loop {
            let opt = {
                let mut clt_r_grd = clt.reader.lock().await;
                clt_r_grd.recv().await?
            };
            match opt {
                Some(msg) => {
                    info!("{:?} RECV: {:?}", clt.con_id, msg);
                    // TODO echo for now
                    clt.writer.lock().await.send::<125>(&msg).await?; // TODO msg size
                }
                None => {
                    return Ok(()); // clean exist
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use soupbintcp4::prelude::{NoPayload, SoupBinHandler, SoupBinMsg};

    use super::*;
    use crate::unittest::setup;

    #[tokio::test]
    async fn test_clt_not_connected() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        let timeout = Duration::from_secs_f32(0.05);
        let clt = Clt::<SoupBinHandler<NoPayload>>::new(addr, timeout, timeout / 5).await;

        info!("{:?}", clt);
        assert!(clt.is_err())
    }
    #[tokio::test]
    async fn test_clt() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        let timeout = Duration::from_secs(5);
        let mut clt = Clt::<SoupBinHandler<NoPayload>>::new(addr, timeout, timeout / 5)
            .await
            .unwrap();

        let msg = SoupBinMsg::dbg(b"hello world");
        clt.send::<1024>(&msg).await.unwrap();
        info!("{} sent msg: {:?}", clt, msg);

        sleep(Duration::from_secs(1)).await;
    }
}
