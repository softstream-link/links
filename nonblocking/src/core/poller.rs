use std::{
    io::{self, Error},
    thread::Builder,
};

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

pub struct PollHandler<
    M: Messenger+'static,
    C: CallbackRecvSend<M>+'static,
    const MAX_MSG_SIZE: usize,
> {
    poll: Poll,
    serviceable: Slab<Serviceable<M, C, MAX_MSG_SIZE>>,
    events: Events,
}
impl<M: Messenger+Send+Sync, C: CallbackRecvSend<M>+Send+Sync, const MAX_MSG_SIZE: usize>
    PollHandler<M, C, MAX_MSG_SIZE>
{
    pub fn add(&mut self, acceptor: PoolCltAcceptor<M, C, MAX_MSG_SIZE>) -> io::Result<()> {
        self.add_serviceable(Serviceable::PoolCltAcceptor(acceptor))
    }
    pub fn spawn(mut self, name: &str) {
        Builder::new()
            .name(format!("{}-Thread", name).to_owned())
            .spawn(move || loop {
                self.service().unwrap();
            })
            .unwrap();
    }

    fn add_serviceable(&mut self, recver: Serviceable<M, C, MAX_MSG_SIZE>) -> io::Result<()> {
        let key = self.serviceable.insert(recver);
        match self.serviceable[key] {
            Serviceable::CltRecver(ref mut recver) => {
                self.poll
                    .registry()
                    .register(*recver.source(), Token(key), mio::Interest::READABLE)
            }
            Serviceable::PoolCltAcceptor(ref mut acceptor) => self.poll.registry().register(
                *acceptor.source(),
                Token(key),
                mio::Interest::READABLE,
            ),
        }
    }

    pub fn service(&mut self) -> Result<(), Error> {
        use PollEventStatus::*;
        use Serviceable::*;
        self.poll.poll(&mut self.events, None).unwrap();

        let mut at_least_one_completed = true;
        while at_least_one_completed {
            at_least_one_completed = false;

            for event in self.events.iter() {
                let token = event.token().into();
                match self.serviceable.get_mut(token) {
                    Some(CltRecver(recver)) => match recver.on_event() {
                        Ok(Completed) => {
                            at_least_one_completed = true;
                            continue;
                        }
                        Ok(WouldBlock) => continue,
                        Ok(Terminate) => {
                            if log_enabled!(Level::Info) {
                                info!("Clean, service loop termination recver: {}", recver);
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(token);
                        }
                        Err(e) => {
                            if log_enabled!(Level::Info) {
                                info!(
                                    "Error, service loop termination recver: {}, error: {}",
                                    recver, e
                                );
                            }
                            self.poll.registry().deregister(*recver.source())?;
                            self.serviceable.remove(token);
                        }
                    },
                    Some(PoolCltAcceptor(acceptor)) => match acceptor.accept_recver() {
                        Ok(AcceptStatus::Accepted(recver)) => {
                            let key = self.serviceable.insert(CltRecver(recver));
                            if let CltRecver(ref mut recver) = self.serviceable[key] {
                                self.poll.registry().register(
                                    *recver.source(),
                                    Token(key),
                                    mio::Interest::READABLE,
                                )?;
                            }
                            at_least_one_completed = true;
                        }
                        Ok(AcceptStatus::WouldBlock) => {}
                        Err(e) => {
                            if log_enabled!(Level::Info) {
                                info!(
                                    "Error, service loop termination acceptor: {}, error: {}",
                                    acceptor, e
                                );
                            }
                            self.poll.registry().deregister(*acceptor.source())?;
                            self.serviceable.remove(token);
                        }
                    },
                    None => {} // possible when the serviceable is removed with error or terminate from the service loop but other serviceable are still responding in this iteration
                }
            }
        }
        Ok(())
    }
}
impl<M: Messenger, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize> Default
    for PollHandler<M, C, MAX_MSG_SIZE>
{
    fn default() -> Self {
        Self {
            poll: Poll::new().expect("Failed to create Poll"),
            serviceable: Slab::new(),
            events: Events::with_capacity(1024),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{num::NonZeroUsize, thread::sleep, time::Duration};

    use links_core::unittest::setup::{
        self,
        messenger::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        messenger_old::CltTestMessenger,
        model::{TestCltMsg, TestCltMsgDebug},
    };
    use super::PollHandler;
    use crate::prelude::*;

    #[test]
    fn test_poller() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<SvcTestMessenger>::new_ref(), // TODO callback shall not be Arc, but an owned and create a wrap for arc
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc"),
        )
        .unwrap();

        let mut clt = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(
            addr,
            setup::net::default_connect_timeout(),
            setup::net::default_connect_retry_after(),
            DevNullCallback::<CltTestMessenger>::new_ref(),
            Some("unittest/clt"),
        )
        .unwrap();

        let (acceptor, _, _sender_pool) = svc.into_split();

        let mut poll_handler = PollHandler::<_, _, TEST_MSG_FRAME_SIZE>::default();
        poll_handler.add(acceptor).unwrap();

        poll_handler.spawn("Svc-Poll");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..4 {
            clt.send_busywait(&mut msg).unwrap();
        }

        sleep(Duration::from_millis(200));
    }
}
