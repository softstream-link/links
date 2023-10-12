use std::{
    io::{self, Error},
    thread::Builder,
};

use log::{info, log_enabled, Level};
use mio::{Events, Poll, Token};
use slab::Slab;

use crate::prelude::*;
pub enum Serviceable {
    Acceptor(Box<dyn PollAccept>),
    Recver(Box<dyn PollObject>),
}

pub struct PollHandler {
    poll: Poll,
    serviceable: Slab<Serviceable>,
    events: Events,
}
impl PollHandler {
    pub fn add(&mut self, acceptor: Box<dyn PollAccept>) -> io::Result<()> {
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

    fn add_serviceable(&mut self, recver: Serviceable) -> io::Result<()> {
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
                        Ok(Some(recver)) => {
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
                        Ok(None) => {}
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
impl Default for PollHandler {
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

    use super::PollHandler;
    use crate::prelude::*;
    use links_core::unittest::setup::{
        self,
        messenger::{SvcTestMessenger, TEST_MSG_FRAME_SIZE},
        messenger_old::CltTestMessenger,
        model::{TestCltMsg, TestCltMsgDebug},
    };

    #[test]
    fn test_poller() {
        setup::log::configure_level(log::LevelFilter::Info);

        let addr = setup::net::rand_avail_addr_port();
        let addr1 = setup::net::rand_avail_addr_port();

        let svc = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr,
            LoggerCallback::<SvcTestMessenger>::new_ref(), // TODO callback shall not be Arc, but an owned and create a wrap for arc
            NonZeroUsize::new(1).unwrap(),
            Some("unittest/svc"),
        )
        .unwrap();

        let svc1 = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(
            addr1,
            LoggerCallback::<CltTestMessenger>::new_ref(), // TODO callback shall not be Arc, but an owned and create a wrap for arc
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
        let (acceptor1, _, _sender_pool1) = svc1.into_split();

        let mut poll_handler = PollHandler::default();
        poll_handler.add(acceptor.into()).unwrap();
        poll_handler.add(acceptor1.into()).unwrap();

        poll_handler.spawn("Recv-Poller");

        let mut msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
        for _ in 0..4 {
            clt.send_busywait(&mut msg).unwrap();
        }

        sleep(Duration::from_millis(200));
    }
}
