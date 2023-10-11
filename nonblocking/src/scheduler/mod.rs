use std::io::{self, Error};

use links_core::prelude::Messenger;
use log::{info, log_enabled, Level};
use mio::{Events, Poll, Token};
use slab::Slab;

use crate::prelude::*;
pub enum Serviceable<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    PoolCltAcceptor(PoolCltAcceptor<M, C, MAX_MSG_SIZE>),
    CltRecver(CltRecver<M, C, MAX_MSG_SIZE>),
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    Serviceable<M, C, MAX_MSG_SIZE>
{
}

pub struct CallBackLoopHandler<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    poll: Poll,
    serviceable: Slab<Serviceable<M, C, MAX_MSG_SIZE>>,
    events: Events,
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>
    CallBackLoopHandler<M, C, MAX_MSG_SIZE>
{
    pub fn add(&mut self, acceptor: PoolCltAcceptor<M, C, MAX_MSG_SIZE>) -> io::Result<()> {
        self.add_serviceable(Serviceable::PoolCltAcceptor(acceptor))
    }

    fn add_serviceable(&mut self, recver: Serviceable<M, C, MAX_MSG_SIZE>) -> io::Result<()> {
        let key = self.serviceable.insert(recver);
        let token = Token(key);
        let registry = self.poll.registry();
        match self.serviceable[key] {
            Serviceable::CltRecver(ref mut recver) => registry.register(
                &mut recver.msg_recver.frm_reader.stream_reader,
                token,
                mio::Interest::READABLE,
            ),
            Serviceable::PoolCltAcceptor(ref mut acceptor) => registry.register(
                &mut acceptor.acceptor.listener,
                token,
                mio::Interest::READABLE,
            ),
        }
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>+Send, const MAX_MSG_SIZE: usize> Default
    for CallBackLoopHandler<M, C, MAX_MSG_SIZE>
{
    fn default() -> Self {
        Self {
            poll: Poll::new().expect("Failed to create Poll"),
            serviceable: Slab::new(),
            events: Events::with_capacity(1024),
        }
    }
}

impl<M: Messenger, C: CallbackRecvSend<M>+Send, const MAX_MSG_SIZE: usize> NonBlockingServiceLoop
    for CallBackLoopHandler<M, C, MAX_MSG_SIZE>
{
    fn service_once(&mut self) -> Result<ServiceLoopStatus, Error> {
        use ServiceLoopStatus::*;
        use Serviceable::*;
        self.poll.poll(&mut self.events, None).unwrap();

        let mut at_least_one_completed = true;
        while at_least_one_completed {
            at_least_one_completed = false;

            for event in self.events.iter() {
                let token = event.token().into();
                match self.serviceable.get_mut(token) {
                    Some(CltRecver(recver)) => match recver.service_once() {
                        Ok(Completed) => {
                            at_least_one_completed = true;
                            continue;
                        }
                        Ok(WouldBlock) => continue,
                        Ok(Terminate) => {
                            if log_enabled!(Level::Info) {
                                info!("Clean, service loop termination recver: {}", recver);
                            }
                            self.poll
                                .registry()
                                .deregister(&mut recver.msg_recver.frm_reader.stream_reader)?;
                            self.serviceable.remove(token);
                        }
                        Err(e) => {
                            if log_enabled!(Level::Info) {
                                info!(
                                    "Error, service loop termination recver: {}, error: {}",
                                    recver, e
                                );
                            }
                            self.poll
                                .registry()
                                .deregister(&mut recver.msg_recver.frm_reader.stream_reader)
                                .expect("Failed to deregister recver");
                            self.serviceable.remove(token);
                        }
                    },
                    Some(PoolCltAcceptor(acceptor)) => {
                        match acceptor.pool_accept_sender_return_recver() {
                            Ok(Some(recver)) => {
                                let key = self.serviceable.insert(CltRecver(recver));
                                if let CltRecver(ref mut recver) = self.serviceable[key] {
                                    self.poll.registry().register(
                                        &mut recver.msg_recver.frm_reader.stream_reader,
                                        Token(key),
                                        mio::Interest::READABLE,
                                    )?;
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                if log_enabled!(Level::Info) {
                                    info!(
                                        "Error, service loop termination acceptor: {}, error: {}",
                                        acceptor, e
                                    );
                                }
                                self.poll
                                    .registry()
                                    .deregister(&mut acceptor.acceptor.listener)
                                    .expect("Failed to deregister acceptor");
                                self.serviceable.remove(token);
                            }
                        }
                    }
                    None => {} // possible when the serviceable is removed with error or terminate from the service loop but other serviceable are still responding in this iteration
                }
            }
        }

        Ok(Completed) // never return WouldBlock as we are using poll
    }
}

#[cfg(test)]
mod test {
    use std::{
        num::NonZeroUsize,
        thread::{sleep, Builder},
        time::Duration,
    };

    use links_core::unittest::setup::{
        self,
        messenger::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        messenger_old::CltTestMessenger,
        model::{TestCltMsg, TestCltMsgDebug},
    };

    use super::CallBackLoopHandler;
    use crate::prelude::*;

    #[test]
    fn test_scheduler() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<SvcTestMessenger>::new_ref(), // TODO callback shall not be Arc, but an owned and create a wrap for arc
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc"),
        )
        .unwrap();

        let mut handler = CallBackLoopHandler::<_, _, TEST_MSG_FRAME_SIZE>::default();

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::<CltTestMessenger>::new_ref(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        handler.add(acceptor).unwrap();

        Builder::new()
            .name("Svc-Blah".to_owned())
            .spawn(move || loop {
                handler.service_once().unwrap();
            })
            .unwrap();

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..4 {
            clt.send_busywait(&mut msg).unwrap();
        }
        drop(clt);
        sleep(Duration::from_millis(200));
    }
}
