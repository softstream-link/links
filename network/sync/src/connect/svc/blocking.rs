// use std::{
//     io::{Error, ErrorKind},
//     net::TcpListener,
//     sync::{mpsc::{Sender, Receiver}, Arc}, fmt::Display,
// };

// use links_network_core::prelude::{CallbackSendRecv, ConId, Messenger, CallbackRecv};
// use log::{debug, log_enabled, warn};
// use slab::Slab;

// use crate::prelude_blocking::{AcceptClt, Clt, CltRecver, CltSender};

// #[derive(Debug)]
// pub struct SvcAcceptor<
//     M: Messenger+'static,
//     C: CallbackSendRecv<M>+'static,
//     const MAX_MSG_SIZE: usize,
// > {
//     tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
//     tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
//     listener: TcpListener,
//     callback: Arc<C>,
//     con_id: ConId,
// }
// impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE>
//     for SvcAcceptor<M, C, MAX_MSG_SIZE>
// {
//     fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
//         match self.listener.accept() {
//             Ok((stream, addr)) => {
//                 // TODO add rate limiter
//                 let mut con_id = self.con_id.clone();
//                 con_id.set_peer(addr);
//                 if log_enabled!(log::Level::Debug) {
//                     debug!("{} Accepted", con_id);
//                 }
//                 let clt = Clt::<_, _, MAX_MSG_SIZE>::from_stream(
//                     stream,
//                     con_id.clone(),
//                     self.callback.clone(),
//                 );
//                 Ok(clt)
//             }
//             Err(e) => Err(e),
//         }
//     }
// }

// pub struct SvcRecver<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
//     rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
//     svc_recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
// }
// impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> SvcRecver<M, C, MAX_MSG_SIZE> {
//     #[inline]
//     fn service_once_rx_queue(&mut self) -> Result<(), Error> {
//         match self.rx_recver.try_recv() {
//             Ok(recver) => {
//                 if self.svc_recvers.len() < self.svc_recvers.capacity() {
//                     if log_enabled!(log::Level::Debug) {
//                         debug!("Adding recver: {} to {}", recver, self);
//                     }
//                     self.svc_recvers.insert(recver);
//                 } else {
//                     warn!("Dropping recver: {}, {} at capacity", recver, self,);
//                 }
//                 Ok(())
//             }
//             Err(std::sync::mpsc::TryRecvError::Empty) => Ok(()),
//             Err(e) => Err(Error::new(ErrorKind::Other, e)),
//         }
//     }
//     #[inline]
//     fn service_once_recvers(&mut self) -> Result<(), Error> {
//         let (mut dead_key, mut dead_connection) = (0, false);
//         for (key, clt) in self.svc_recvers.iter_mut() {
//             match clt.service_once() {
//                 Ok(ServiceLoopStatus::Continue) => {}
//                 Ok(ServiceLoopStatus::Stop) => {
//                     dead_connection = true;
//                     dead_key = key;
//                     break;
//                 }
//                 Err(e) => {
//                     warn!(
//                         "recver #{} is dead {} and will be dropped.  error: {}",
//                         key, clt, e
//                     );
//                     dead_connection = true;
//                     dead_key = key;
//                     break;
//                 }
//             };
//         }
//         // TODO fix this so that we only iterate each clt once
//         if dead_connection {
//             self.svc_recvers.remove(dead_key);
//             self.service_once_recvers()?;
//         }
//         Ok(())
//     }
// }
// impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
//     for SvcRecver<M, C, MAX_MSG_SIZE>
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "SvcRecver<{} of {} [{}]>",
//             self.svc_recvers.len(),
//             self.svc_recvers.capacity(),
//             self.svc_recvers
//                 .iter()
//                 .map(|(_, clt)| format!("{}", clt))
//                 .collect::<Vec<_>>()
//                 .join(","),
//         )
//     }
// }


// pub struct Svc<M: Messenger+'static, C: CallbackSendRecv<M>+'static, const MAX_MSG_SIZE: usize> {
//     acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
//     recver: SvcRecver<M, C, MAX_MSG_SIZE>,
//     sender: SvcSender<M, C, MAX_MSG_SIZE>,
// }
// impl<M: Messenger, C: CallbackSendRecv<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE>
//     for Svc<M, C, MAX_MSG_SIZE>
// {
//     fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
//         self.acceptor.accept()
//     }
// }
