// use std::{
//     io::{Read, Write},
//     net::{TcpListener, TcpStream},
//     thread::{sleep, spawn, JoinHandle},
//     time::Duration,
// };

// use log::info;

// use super::callback::ReadCallback;

// #[derive(Debug)]
// pub struct Svc<'con, CALLBACK>
// where
//     CALLBACK: ReadCallback,
// {
//     // listener: TcpListener,
//     local_addr: String,
//     callback: &'con CALLBACK,
//     acceptor_join_handle: JoinHandle<()>,
// }

// impl<'con, CALLBACK> Drop for Svc<'con, CALLBACK>
// where
//     CALLBACK: ReadCallback,
// {
//     fn drop(&mut self) {
//         info!("dropping Svc");
//     }
// }
// impl<'con, CALLBACK> Svc<'con, CALLBACK>
// where
//     CALLBACK: ReadCallback,
// {
//     pub fn new(local_addr: String, callback: &'con CALLBACK) -> Result<Self, std::io::Error> {
//         let acceptor_join_handle = Self::start_listner_thread(local_addr.clone())?;
//         let svc = Svc {
//             local_addr,
//             callback,
//             acceptor_join_handle,
//         };
//         Ok(svc)
//     }

//     fn start_listner_thread(local_addr: String) -> Result<JoinHandle<()>, std::io::Error> {
//         // self.listener.set_nonblocking(true).expect("Failed to set non-blocking: true");
//         // let acceptor = self
//         //     .listener
//         //     .try_clone()
//         //     .expect("Failed to clone listener for new thread ");
//         // let local_addr = self.local_addr.clone();
//         let acceptor_join_handle = spawn(move || {
//             let listener = TcpListener::bind(local_addr).unwrap(); // panic if failed to bind

//             for stream in listener.incoming() {
//                 match stream {
//                     Ok(s) => {
//                         info!("new connection: {:?}", s);
//                         sleep(Duration::from_secs(5));
//                         // handle_client(s);
//                     }
//                     // Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
//                     //         info!("Error: {}", e);
//                     // },
//                     Err(e) => panic!("encountered IO error: {}", e),
//                 }
//             }
//             // loop {
//             //     info!("waiting for connection...");
//             //     let (mut stream, addr) = acceptor.accept().unwrap();
//             //     info!("accepted connection from {:?}", addr);
//             //     info!("stream: {:?}", stream);
//             //     let mut buf = [0_u8; 1024];
//             //     let n = stream.read(&mut buf[..]).unwrap();
//             //     info!("read {} bytes, buf: {:?}", n, &buf[..n]);
//             //     let n = stream.write(&buf[..n]).unwrap();
//             //     info!("wrote {} bytes, buf: {:?}", n, &buf[..n]);
//             // }

//             // callback.on_accept(&stream, &addr);
//         });
//         Ok(acceptor_join_handle)
//     }

//     // fn handle_clt_stream(mut stream: TcpStream, callback: &'con CALLBACK) {
//     //     let mut buf = [0_u8; 1024];
//     //     let n = stream.read(&mut buf[..]).unwrap();
//     //     info!("read {} bytes, buf: {:?}", n, &buf[..n]);
//     //     let n = stream.write(&buf[..n]).unwrap();
//     //     info!("wrote {} bytes, buf: {:?}", n, &buf[..n]);
//     // }
// }

// #[cfg(test)]
// mod test {
//     use std::io::stdin;

//     use super::*;
//     use crate::{tcp::callback::LoggerCallback, unittest::setup};
//     use log::info;

//     #[test]
//     fn test_svc() {
//         setup::log::configure();

//         let addr = "0.0.0.0:5000";
//         let callback = LoggerCallback {};
//         let svc = Svc::new(addr.to_owned(), &callback).unwrap();
//         info!("svc: {:?}", svc);
//         stdin().read_line(&mut String::new()).unwrap();

//         // console
//     }
// }
