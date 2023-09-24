use std::{
    fmt::Display,
    io::{Error, ErrorKind},
    net::TcpListener,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use links_network_core::prelude::{CallbackRecv, CallbackRecvSend, CallbackSend, ConId, Messenger};
use log::{debug, log_enabled, warn};
use slab::Slab;

use crate::prelude_blocking::{AcceptClt, Clt, CltRecver, CltSender};

#[derive(Debug)]
pub struct SvcAcceptor<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    tx_recver: Sender<CltRecver<M, C, MAX_MSG_SIZE>>,
    tx_sender: Sender<CltSender<M, C, MAX_MSG_SIZE>>,
    listener: TcpListener,
    callback: Arc<C>,
    con_id: ConId,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // TODO add rate limiter
                let mut con_id = self.con_id.clone();
                con_id.set_peer(addr);
                if log_enabled!(log::Level::Debug) {
                    debug!("{} Accepted", con_id);
                }
                let clt = Clt::<_, _, MAX_MSG_SIZE>::from_stream(
                    stream,
                    con_id.clone(),
                    self.callback.clone(),
                );
                Ok(clt)
            }
            Err(e) => Err(e),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE>
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        self.listener.set_nonblocking(false)?;
        SvcAcceptor::accept(&self)
    }
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.listener.set_nonblocking(true)?;
        match SvcAcceptor::accept(&self) {
            Ok(clt) => Ok(Some(clt)),
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for SvcAcceptor<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} SvcAcceptor", self.con_id)
    }
}

#[derive(Debug)]
pub struct SvcRecver<M: Messenger+'static, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> {
    rx_recver: Receiver<CltRecver<M, C, MAX_MSG_SIZE>>,
    svc_recvers: Slab<CltRecver<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> SvcRecver<M, C, MAX_MSG_SIZE> {
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Error> {
        match self.rx_recver.try_recv() {
            Ok(recver) => {
                if self.svc_recvers.len() < self.svc_recvers.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding recver: {} to {}", recver, self);
                    }
                    self.svc_recvers.insert(recver);
                } else {
                    warn!("Dropping recver: {}, {} at capacity", recver, self,);
                }
                Ok(())
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(()),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
    // #[inline]
    // fn service_once_recvers(&mut self) -> Result<(), Error> {
    //     let (mut dead_key, mut dead_connection) = (0, false);
    //     for (key, clt) in self.svc_recvers.iter_mut() {
    //         match clt.service_once() {
    //             Ok(ServiceLoopStatus::Continue) => {}
    //             Ok(ServiceLoopStatus::Stop) => {
    //                 dead_connection = true;
    //                 dead_key = key;
    //                 break;
    //             }
    //             Err(e) => {
    //                 warn!(
    //                     "recver #{} is dead {} and will be dropped.  error: {}",
    //                     key, clt, e
    //                 );
    //                 dead_connection = true;
    //                 dead_key = key;
    //                 break;
    //             }
    //         };
    //     }
    //     // TODO fix this so that we only iterate each clt once
    //     if dead_connection {
    //         self.svc_recvers.remove(dead_key);
    //         self.service_once_recvers()?;
    //     }
    //     Ok(())
    // }
}
impl<M: Messenger, C: CallbackRecv<M>, const MAX_MSG_SIZE: usize> Display
    for SvcRecver<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SvcRecver<{} of {} [{}]>",
            self.svc_recvers.len(),
            self.svc_recvers.capacity(),
            self.svc_recvers
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

#[derive(Debug)]
pub struct SvcSender<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> {
    rx_sender: Receiver<CltSender<M, C, MAX_MSG_SIZE>>,
    svc_senders: Slab<CltSender<M, C, MAX_MSG_SIZE>>,
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> SvcSender<M, C, MAX_MSG_SIZE> {
    #[inline]
    fn service_once_rx_queue(&mut self) -> Result<(), Error> {
        match self.rx_sender.try_recv() {
            Ok(sender) => {
                if self.svc_senders.len() < self.svc_senders.capacity() {
                    if log_enabled!(log::Level::Debug) {
                        debug!("Adding sender: {} to {}", sender, self);
                    }
                    self.svc_senders.insert(sender);
                } else {
                    warn!("Dropping sender: {}, {} at capacity", sender, self);
                }
                Ok(())
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(()),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}
impl<M: Messenger, C: CallbackSend<M>, const MAX_MSG_SIZE: usize> Display
    for SvcSender<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SvcSender<{} of {} [{}]>",
            self.svc_senders.len(),
            self.svc_senders.capacity(),
            self.svc_senders
                .iter()
                .map(|(_, clt)| format!("{}", clt))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

pub struct Svc<M: Messenger+'static, C: CallbackRecvSend<M>+'static, const MAX_MSG_SIZE: usize> {
    acceptor: SvcAcceptor<M, C, MAX_MSG_SIZE>,
    recver: SvcRecver<M, C, MAX_MSG_SIZE>,
    sender: SvcSender<M, C, MAX_MSG_SIZE>,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Svc<M, C, MAX_MSG_SIZE> {
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        max_connections: usize, // TODO this arg needs better name
        name: Option<&str>,
    ) -> Result<Self, Error> {
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;

        let (tx_recver, rx_recver) = channel();
        let (tx_sender, rx_sender) = channel();

        let acceptor = SvcAcceptor {
            tx_recver,
            tx_sender,
            listener,
            callback,
            con_id: ConId::svc(name, addr, None),
        };
        let svc_recver = SvcRecver {
            rx_recver,
            svc_recvers: Slab::with_capacity(max_connections),
        };
        let svc_sender = SvcSender {
            rx_sender,
            svc_senders: Slab::with_capacity(max_connections),
        };
        Ok(Self {
            acceptor,
            recver: svc_recver,
            sender: svc_sender,
        })
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> AcceptClt<M, C, MAX_MSG_SIZE>
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn accept(&self) -> Result<Clt<M, C, MAX_MSG_SIZE>, Error> {
        self.acceptor.accept()
    }
    fn accept_nonblocking(&self) -> Result<Option<Clt<M, C, MAX_MSG_SIZE>>, Error> {
        self.acceptor.accept_nonblocking()
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Display
    for Svc<M, C, MAX_MSG_SIZE>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} Svc<{}, {}, {}>",
            self.acceptor.con_id, self.acceptor, self.recver, self.sender
        )
    }
}

#[cfg(test)]
#[cfg(any(test, feature = "unittest"))]
mod test {

    use links_network_core::prelude::{DevNullCallback, LoggerCallback};
    use links_testing::unittest::setup;
    use log::info;

    use crate::{
        prelude_blocking::*,
        unittest::setup::framer::{TestSvcMsgProtocol, TEST_MSG_FRAME_SIZE},
    };

    #[test]
    fn test_svc_not_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            DevNullCallback::<TestSvcMsgProtocol>::new_ref(),
            2,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);
    }

    #[test]
    fn test_svc_clt_connected() {
        setup::log::configure();
        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<TestSvcMsgProtocol>::new_ref(),
            2,
            Some("unittest"),
        )
        .unwrap();
        info!("svc: {}", svc);
        let res_accept_nonblocking = svc.accept_nonblocking();
        info!("res_accept_nonblocking: {:?}", res_accept_nonblocking);
        assert!(match res_accept_nonblocking {
            Ok(None) => true,
            _ => false,
        });

        let clt_initiator = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            LoggerCallback::<TestSvcMsgProtocol>::new_ref(),
            Some("unittest"),
        )
        .unwrap();
        info!("clt_initiator: {}", clt_initiator);

        let clt_acceptor = svc.accept().unwrap();
        info!("svc: {}", svc);
        info!("clt_acceptor: {}", clt_acceptor);
    }
}
