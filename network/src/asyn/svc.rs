// use std::{
//     error::Error,
//     sync::{Arc, Mutex},
// };

// use framing::MessageHandler;
// use log::{error, info};
// use tokio::net::TcpListener;

// use crate::asyn::clt::{Clt, CltWriter, ConId};

// use super::con_msg::{StreamMessenderReader, StreamMessenderWriter};

// pub type SvcReaderRef<HANDLER> = Arc<Mutex<Option<StreamMessenderReader<HANDLER>>>>;
// pub type SvcWriterRef<HANDLER> = Arc<Mutex<Option<StreamMessenderWriter<HANDLER>>>>;

// #[derive(Debug)]
// pub struct Svc<HANDLER: MessageHandler> {
//     // reader: SvcReaderRef<HANDLER>,
//     // writer: SvcWriterRef<HANDLER>,
//     phantom: std::marker::PhantomData<HANDLER>,
// }

// impl<HANDLER> Svc<HANDLER>
// where
//     HANDLER: MessageHandler,
// {
//     pub async fn new(addr: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
//         let con_id = ConId::Svc(addr.to_owned());
//         let lis = TcpListener::bind(&addr).await?;
//         info!("{:?} bound successfully", con_id);

//         tokio::spawn(async move {
//             info!("{:?} accept loop started", con_id);
//             match Self::run(lis).await {
//                 Ok(()) => info!("{:?} accept loop stopped", con_id),
//                 Err(err) => error!("{:?} accept loop exit err: {:?}", con_id, err),
//             }
//         });
//         Ok(())
//     }
//     async fn run(lis: TcpListener) -> Result<(), Box<dyn Error + Send + Sync>> {
//         loop {
//             let (stream, _) = lis.accept().await.unwrap();
//             let con_id = ConId::Svc(format!(
//                 "{:?}<-{:?}",
//                 stream.local_addr()?,
//                 stream.peer_addr()?,
//             ));
//             let _: CltWriter<HANDLER> = Clt::<HANDLER>::from_stream(stream, con_id.clone()).await;
//             // info!("{:?} STREAM STOPPED", con_id);
//         }
//     }
// }

// #[cfg(test)]
// mod test {
//     use soupbintcp4::prelude::{NoPayload, SoupBinHandler};

//     use super::*;
//     use crate::unittest::setup;
//     use tokio::time::{sleep, Duration};

//     #[tokio::test]
//     async fn test_svc() {
//         setup::log::configure();
//         let addr = &setup::net::default_addr();
//         let svc = Svc::<SoupBinHandler<NoPayload>>::new(addr).await;
//         info!("svc: {:?}", svc);
//         // svc.send(SoupBinMsg::dbg(b"hello world from server!")).await;
//         sleep(Duration::from_secs(100)).await;
//     }
// }
