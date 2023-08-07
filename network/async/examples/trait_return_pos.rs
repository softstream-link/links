// #![feature(async_fn_in_trait)]
// #![feature(return_position_impl_trait_in_trait)]
// use std::future::Future;


// use bytes::BytesMut;
// use tokio::{
//     io::AsyncReadExt,
//     net::{
//         tcp::{OwnedReadHalf, OwnedWriteHalf},
//         TcpListener,
//     },
// };

// trait ProtocolInitHandler: Sync + Send + 'static {
//     fn login_sequence<'s, HANDLER: ProtocolInitHandler>(&'s self, svc: &'s Svc<HANDLER>) -> impl Future<Output = ()> + Send + '_;
// }

// struct SvcProtocolInitHandler;
// impl SvcProtocolInitHandler {
//     fn new() -> Self {
//         Self
//     }
// }
// impl ProtocolInitHandler for SvcProtocolInitHandler {
//     async fn login_sequence<'s, HANDLER: ProtocolInitHandler>(&'s self, svc: &'s Svc<HANDLER>) {
//         println!("login sequence do something");
//     }

// }


// struct Svc<HANDLER>
// where
//     HANDLER: ProtocolInitHandler,
// {
//     reader: OwnedReadHalf,
//     writer: OwnedWriteHalf,
//     phantom: std::marker::PhantomData<HANDLER>,
// }
// impl<HANDLER> Svc<HANDLER>
// where
//     HANDLER: ProtocolInitHandler,
// {
//     async fn bind(addr: &str, handler: HANDLER) {
//         tokio::spawn({
//             let addr = addr.to_owned();
//             async move {
//                 let listener = TcpListener::bind(addr).await.unwrap();
//                 println!("pending accept");
//                 let (stream, _) = listener.accept().await.unwrap();
//                 let (read, write) = stream.into_split();
//                 let svc = Self {
//                     reader: read,
//                     writer: write,
//                     phantom: std::marker::PhantomData,
//                 };
//                 Self::service_loop(svc, handler).await;
//             }
//         })
//         .await
//         .unwrap();
//     }
//     async fn service_loop(mut svc: Svc<HANDLER>, handler: HANDLER) {
//         println!("start login sequence");
//         handler.login_sequence(&svc).await;
//         println!("start service loop");
//         loop {
//             let mut buf = BytesMut::new();
//             let n = svc.reader.read_buf(&mut buf);
//             println!("read {:?} bytes", n);
//         }
//     }
// }

#[tokio::main]
async fn main() {
    // let addr = "0.0.0.0:8080";
    // let handler = SvcProtocolInitHandler::new();
    // let _ = Svc::bind(addr, handler);
}
