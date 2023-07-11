use std::{error::Error, fmt::Debug};

use framing::prelude::*;
use tokio::net::TcpStream;

use super::con_frame::ConnectionFramed;

#[derive(Debug)]
pub struct ConnectionMessenger<MHANDLER, const MAX_MSG_SIZE: usize>
where
    MHANDLER: MessageHandler<MAX_MSG_SIZE>,
{
    con: ConnectionFramed<MHANDLER::FHANDLER>,
}

impl<MHANDLER, const MAX_MSG_SIZE: usize> ConnectionMessenger<MHANDLER, MAX_MSG_SIZE>
where
    MHANDLER: MessageHandler<MAX_MSG_SIZE>,
{
    pub fn new(socket: TcpStream) -> ConnectionMessenger<MHANDLER, MAX_MSG_SIZE> {
        ConnectionMessenger {
            con: ConnectionFramed::with_capacity(socket, MAX_MSG_SIZE * 16),
        }
    }
    pub fn with_capacity(
        socket: TcpStream,
        capacity: usize,
    ) -> ConnectionMessenger<MHANDLER, MAX_MSG_SIZE> {
        ConnectionMessenger {
            con: ConnectionFramed::with_capacity(socket, capacity),
        }
    }

    pub async fn send(
        &mut self,
        msg: &MHANDLER::MSG,
    ) -> std::result::Result<(), Box<dyn Error + Send + Sync>> {
        let (bytes, size) = MHANDLER::from_msg(msg).unwrap(); // TODO handle error
        self.con.write_frame(&bytes[..size]).await?;
        Ok(())
    }

    pub async fn recv(
        &mut self,
    ) -> std::result::Result<Option<MHANDLER::MSG>, Box<dyn Error + Send + Sync>> {
        let frame = self.con.read_frame().await?;
        if let Some(frm) = frame {
            let msg = MHANDLER::into_msg(frm).unwrap();
            Ok(Some(msg))
        } else {
            Ok(None)
        }
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
        let addr = setup::net::svc_default_addr();
        type SoupBinVec = SoupBin<VecPayload>;
        type SoupBinDefaultHanlder = SoupBinMessageHandler<VecPayload>;
        type SoupBinConnection = ConnectionMessenger<SoupBinDefaultHanlder, 1024>;
        let svc = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let listener = TcpListener::bind(addr).await.unwrap();

                let (socket, _) = listener.accept().await.unwrap();
                let mut svc = SoupBinConnection::new(socket);
                info!("svc - conn: {:?}", svc);

                loop {
                    let msg = svc.recv().await.unwrap();
                    match msg {
                        Some(msg) => {
                            info!("svc - msg: {:?}", msg);
                            let msg = SoupBinVec::dbg(b"hello world from server!");
                            info!("svc - msg: {:?}", msg);
                            svc.send(&msg).await.unwrap();
                        }
                        None => {
                            info!("svc - msg: None - Connection Closed by Client");
                            break;
                        }
                    }
                }
            })
        };
        let clt = {
            let addr = addr.clone();
            tokio::spawn(async move {
                let socket = TcpStream::connect(addr).await.unwrap();
                let mut clt = SoupBinConnection::new(socket);
                info!("clt - conn: {:?}", clt);
                let msg = SoupBinVec::dbg(b"hello world from client!");
                info!("clt - msg: {:?}", msg);
                clt.send(&msg).await.unwrap();
                let msg = clt.recv().await.unwrap();
                info!("clt - msg: {:?}", msg);
            })
        };
        clt.await.unwrap();
        svc.await.unwrap();
    }
}
