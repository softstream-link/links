use std::{
    fmt::Display,
    io::{self, Error},
    thread::Builder,
};

use log::{info, log_enabled, Level};
use mio::{Events, Poll, Token};
use slab::Slab;

use crate::{core::PollAcceptStatic, prelude::*};
pub enum Serviceable<R: PollRecv, A: PollAcceptStatic<R>> {
    Acceptor(A),
    Recver(R),
}

pub struct PollHandler<R: PollRecv, A: PollAcceptStatic<R>> {
    poll: Poll,
    serviceable: Slab<Serviceable<R, A>>,
    events: Events,
}
impl<R: PollRecv, A: PollAcceptStatic<R>> PollHandler<R, A> {
    pub fn add(&mut self, acceptor: A) -> io::Result<()> {
        self.add_serviceable(Serviceable::Acceptor(acceptor))
    }
    pub fn spawn(mut self, name: &str) {
        Builder::new()
            .name(format!("{}-Thread", name).to_owned())
            .spawn(move || loop {
                self.service().unwrap();
            })
            .unwrap();
    }

    fn add_serviceable(&mut self, recver: Serviceable<R, A>) -> io::Result<()> {
        let key = self.serviceable.insert(recver);
        match self.serviceable[key] {
            Serviceable::Recver(ref mut recver) => {
                self.poll
                    .registry()
                    .register(*recver.source(), Token(key), mio::Interest::READABLE)
            }
            Serviceable::Acceptor(ref mut acceptor) => self.poll.registry().register(
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
                    Some(Recver(recver)) => match recver.on_event() {
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
                    Some(Acceptor(acceptor)) => match acceptor.poll_accept() {
                        Ok(AcceptStatus::Accepted(recver)) => {
                            let key = self.serviceable.insert(Recver(recver));
                            if let Recver(ref mut recver) = self.serviceable[key] {
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
impl<R: PollRecv, A: PollAcceptStatic<R>> Default for PollHandler<R, A> {
    fn default() -> Self {
        Self {
            poll: Poll::new().expect("Failed to create Poll"),
            serviceable: Slab::new(),
            events: Events::with_capacity(1024),
        }
    }
}

pub struct PollAcceptDyn(Box<dyn PollAcceptStatic<Box<dyn PollRecv>>>);

impl PollAcceptStatic<Box<dyn PollRecv>> for PollAcceptDyn {
    fn poll_accept(&mut self) -> io::Result<AcceptStatus<Box<dyn PollRecv>>> {
        self.0.poll_accept()
    }
}
impl PollRecv for PollAcceptDyn {
    fn on_event(&mut self) -> Result<PollEventStatus, Error> {
        self.0.on_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.0.source()
    }
}
impl Display for PollAcceptDyn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PollRecv for Box<dyn PollRecv> {
    fn on_event(&mut self) -> Result<PollEventStatus, Error> {
        self.as_mut().on_event()
    }
    fn source(&mut self) -> Box<&mut dyn mio::event::Source> {
        self.as_mut().source()
    }
}

pub type PollHandlerDyn = PollHandler<Box<dyn PollRecv>, PollAcceptDyn>;
pub type PollHandlerStatic<M, C, const MAX_MSG_SIZE: usize> =
    PollHandler<CltRecver<M, C, MAX_MSG_SIZE>, PoolCltAcceptor<M, C, MAX_MSG_SIZE>>;

#[cfg(test)]
mod test {
    use std::{num::NonZeroUsize, thread::sleep, time::Duration};

    use super::{PollAcceptDyn, PollHandlerDyn, PollHandlerStatic};
    use crate::prelude::*;
    use links_core::unittest::setup::{
        self,
        messenger::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        messenger_old::CltTestMessenger,
        model::{TestCltMsg, TestCltMsgDebug},
    };

    #[test]
    fn test_poller_static() {
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

        let mut poll_handler = PollHandlerStatic::<_, _, TEST_MSG_FRAME_SIZE>::default();
        poll_handler.add(acceptor).unwrap();

        poll_handler.spawn("Svc-Poll");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..4 {
            clt.send_busywait(&mut msg).unwrap();
        }

        sleep(Duration::from_millis(200));
    }

    #[test]
    fn test_poller_dynamic() {
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

        let mut poll_handler = PollHandlerDyn::default();
        poll_handler.add(PollAcceptDyn(Box::new(acceptor))).unwrap();

        poll_handler.spawn("Svc-Poll");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..4 {
            clt.send_busywait(&mut msg).unwrap();
        }

        sleep(Duration::from_millis(200));
    }
}
