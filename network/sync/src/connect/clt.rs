// use std::{
//     error::Error,
//     time::{Duration, Instant}, sync::Arc,
// };

// use links_network_core::prelude::*;
// use log::{debug, info};
// use mio::{net::TcpStream, Interest, Poll, Token, Events};

// pub struct Clt {}

// impl Clt {
//     pub fn connect(
//         addr: &str,
//         // timeout: Duration,
//         // retry_after: Duration,
//         name: Option<&str>,
//         // poll: Arc<Pollel>,
//     ) -> Result<(), Box<dyn Error>> {
//         let mut stream = TcpStream::connect(addr.parse()?)?;

//         // let mut poll = poll;
//         // if let None = poll {
//         //     poll = Some(Poll::new()?);
//         // }
//         let mut poll = Poll::new()?;
//         let mut events = Events::with_capacity(1024);
//         const RW: Interest = Interest::READABLE.add(Interest::WRITABLE);
//         const READ: Interest = Interest::READABLE;;
//         poll.registry().register(&mut stream, Token(0), READ)?;

//         loop{
//             info!("polling...");
//             poll.poll(&mut events, None)?;
//             for event in events.iter() {
//                 info!("event: {:?}", event);
//                 match event.token() {
//                     Token(0) => {
//                         info!("event: {:?}", event);
//                         info!("stream: {:?}", stream);
//                         // info!("accepted connection from: {}", addr);
//                         // poll.registry()
//                         //     .register(&mut stream, Token(1), Interest::READABLE)?;
//                     }
//                     // Token(1) => {
//                     //     info!("received data from: {:?}", event.token());
//                     // }
//                     _ => unreachable!(),
//                 }
//             }
//         }


//         // assert!(timeout > retry_after);
//         // let now = Instant::now();
//         // let mut con_id = ConId::clt(name, None, addr);
//         // while now.elapsed() < timeout {
//         //     let res = TcpStream::connect(addr.parse()?);
//         //     match res {
//         //         Err(e) => {
//         //             debug!("{} connection failed e {:?}", con_id, e);
//         //             std::thread::sleep(retry_after);
//         //         }
//         //         Ok(stream) => {
//         //             info!("{:?}", stream);
//         //             con_id.set_local(stream.local_addr()?);
//         //             con_id.set_peer(stream.peer_addr()?);

//         //             debug!("{} connected", con_id);
//         //             return Ok(());
//         //         }
//         //     }
//         // }
//         Ok(())
//     }
// }

// #[cfg(test)]
// mod test {
//     use std::sync::Arc;

//     use crate::prelude::*;
//     use super::*;
//     use links_testing::unittest::setup;
//     use log::info;
//     #[test]
//     fn test_connect_clt() {
//         setup::log::configure();
//         let pool = Arc::new(Poll::new().unwrap());
//         let addr = "0.0.0.0:8080"; // setup::net::rand_avail_addr_port();
//         let res = Clt::connect(&addr, Some("unittest"), pool);
//         info!("connect res {:?}", res);
//     }
// }
