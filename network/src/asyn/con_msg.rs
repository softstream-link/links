// use std::{
//     error::Error,
//     fmt::{Debug, Display},
// };

// use framing::prelude::*;
// use tokio::net::{
//     tcp::{OwnedReadHalf, OwnedWriteHalf},
//     TcpStream,
// };

// use super::{
//     clt::ConId,
//     con_frame::{StreamFramer, StreamReadFramer, StreamWriteFramer},
// };

// #[derive(Debug)]
// pub struct StreamMessenderWriter<HANDLER: MessageHandler> {
//     con_id: ConId,
//     writer: StreamWriteFramer<HANDLER::FrameHandler>,
// }
// impl<HANDLER: MessageHandler> Display for StreamMessenderWriter<HANDLER> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.con_id)
//     }
// }
// impl<HANDLER: MessageHandler> StreamMessenderWriter<HANDLER> {
//     pub fn new(writer: OwnedWriteHalf, con_id: ConId) -> Self {
//         Self {
//             con_id,
//             writer: StreamWriteFramer::new(writer),
//         }
//     }
//     pub fn with_capacity(writer: OwnedWriteHalf, capacity: usize, con_id: ConId) -> Self {
//         Self {
//             con_id,
//             writer: StreamWriteFramer::with_capacity(writer, capacity),
//         }
//     }
//     pub async fn send<const MAX_MSG_SIZE: usize>(
//         &mut self,
//         msg: &HANDLER::Item,
//     ) -> Result<(), Box<dyn Error + Send + Sync>> {
//         let (bytes, size) = HANDLER::from_msg::<MAX_MSG_SIZE>(msg)?; //TODO fix max size
//         self.writer.write_frame(&bytes[..size]).await?;
//         Ok(())
//     }
// }

// #[derive(Debug)]
// pub struct StreamMessenderReader<HANDLER: MessageHandler> {
//     con_id: ConId,
//     reader: StreamReadFramer<HANDLER::FrameHandler>,
// }
// impl<HANDLER: MessageHandler> Display for StreamMessenderReader<HANDLER> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.con_id)
//     }
// }
// impl<HANDLER: MessageHandler> StreamMessenderReader<HANDLER> {
//     pub fn new(reader: OwnedReadHalf, con_id: ConId) -> Self {
//         Self {
//             con_id,
//             reader: StreamReadFramer::new(reader),
//         }
//     }
//     pub fn with_capacity(reader: OwnedReadHalf, capacity: usize, con_id: ConId) -> Self {
//         Self {
//             con_id,
//             reader: StreamReadFramer::with_capacity(reader, capacity),
//         }
//     }
//     pub async fn recv(&mut self) -> Result<Option<HANDLER::Item>, Box<dyn Error + Send + Sync>> {
//         let frame = self.reader.read_frame().await?;
//         if let Some(frm) = frame {
//             let msg = HANDLER::into_msg(frm)?;
//             Ok(Some(msg))
//         } else {
//             Ok(None)
//         }
//     }
// }

// #[derive(Debug)]
// pub struct StreamMessenger<HANDLER: MessageHandler> {
//     con: StreamFramer<HANDLER::FrameHandler>,
// }

// impl<HANDLER: MessageHandler> StreamMessenger<HANDLER> {
//     pub fn new(stream: TcpStream) -> StreamMessenger<HANDLER> {
//         StreamMessenger {
//             con: StreamFramer::new(stream),
//         }
//     }
//     pub fn with_capacity(stream: TcpStream, capacity: usize) -> StreamMessenger<HANDLER> {
//         StreamMessenger {
//             con: StreamFramer::with_capacity(stream, capacity),
//         }
//     }

//     pub async fn send<const MAX_MSG_SIZE: usize>(
//         // TODO how to package away const into a HANDLER trait
//         &mut self,
//         msg: &HANDLER::Item,
//     ) -> std::result::Result<(), Box<dyn Error + Send + Sync>> {
//         let (bytes, size) = HANDLER::from_msg::<MAX_MSG_SIZE>(msg).unwrap(); // TODO handle error
//         self.con.write_frame(&bytes[..size]).await?;
//         Ok(())
//     }

//     pub async fn recv(
//         &mut self,
//     ) -> std::result::Result<Option<HANDLER::Item>, Box<dyn Error + Send + Sync>> {
//         // TODO proper error
//         let frame = self.con.read_frame().await?;
//         if let Some(frm) = frame {
//             let msg = HANDLER::into_msg(frm).unwrap();
//             Ok(Some(msg))
//         } else {
//             Ok(None)
//         }
//     }
// }

// #[cfg(test)]
// mod test {

//     use super::*;

//     use crate::unittest::setup;
//     use log::info;
//     use soupbintcp4::prelude::*;
//     use tokio::net::TcpListener;

//     #[tokio::test]
//     async fn test_connection() {
//         setup::log::configure();
//         let addr = setup::net::default_addr();
//         type SoupBinVec = SoupBinMsg<VecPayload>;
//         type SoupBinDefaultHanlder = SoupBinHandler<VecPayload>;
//         type SoupBinConnection = StreamMessenger<SoupBinDefaultHanlder>;
//         let svc = {
//             let addr = addr.clone();
//             tokio::spawn(async move {
//                 let listener = TcpListener::bind(addr).await.unwrap();

//                 let (socket, _) = listener.accept().await.unwrap();
//                 let mut svc = SoupBinConnection::new(socket);
//                 info!("svc: {:?}", svc);

//                 loop {
//                     let msg = svc.recv().await.unwrap();
//                     match msg {
//                         Some(msg) => {
//                             info!("svc - msg: {:?}", msg);
//                             let msg = SoupBinVec::dbg(b"hello world from server!");
//                             info!("svc - msg: {:?}", msg);
//                             svc.send::<1024>(&msg).await.unwrap();
//                         }
//                         None => {
//                             info!("svc - msg: None - Connection Closed by Client");
//                             break;
//                         }
//                     }
//                 }
//             })
//         };
//         let clt = {
//             let addr = addr.clone();
//             tokio::spawn(async move {
//                 let socket = TcpStream::connect(addr).await.unwrap();
//                 let mut clt = SoupBinConnection::new(socket);
//                 info!("clt: {:?}", clt);
//                 let msg = SoupBinVec::dbg(b"hello world from client!");
//                 info!("clt - msg: {:?}", msg);
//                 clt.send::<1024>(&msg).await.unwrap();
//                 let msg = clt.recv().await.unwrap();
//                 info!("clt - msg: {:?}", msg);
//             })
//         };
//         clt.await.unwrap();
//         svc.await.unwrap();
//     }
// }
