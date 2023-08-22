use std::{
    collections::HashMap,
    error::Error,
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration, thread,
};

use links_network_core::prelude::*;
use log::{error, info, log_enabled, debug};
// use log::info;
use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token,
};
// use rayon::spawn;

pub type SvcAcceptorRef = Arc<SvcAcceptor>;
pub struct SvcAcceptor {
    poll: Mutex<Poll>,
    listeners: Mutex<HashMap<Token, TcpListener>>,
    streams: Mutex<HashMap<Token, TcpStream>>,
    next_token_id: AtomicUsize,
}
impl SvcAcceptor {
    fn new() -> Self {
        Self {
            poll: Mutex::new(Poll::new().unwrap()),
            listeners: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
            next_token_id: AtomicUsize::new(0),
        }
    }
    pub fn new_ref(name: &str) -> Arc<Self> {
        let x = Arc::new(Self::new());
        thread::Builder::new().name(format!("Thread-{}", name)).spawn({
            let x = x.clone();
            move || x.service_loop()
        }).unwrap();
        x

    }
    fn next_token(&self) -> Token {
        let next_token_id = self.next_token_id.fetch_add(1, Ordering::SeqCst);
        let token = Token(next_token_id);
        token
    }
    pub fn register(
        &self,
        listener: TcpListener,
    ) -> Result<Token, Box<dyn Error+Send+Sync+'static>> {
        let token = Token(self.next_token_id.load(Ordering::SeqCst));
        self.next_token_id.fetch_add(1, Ordering::SeqCst);

        let mut listeners = self.listeners.lock().expect("Poisoned lock");
        listeners.insert(token, listener);

        self.poll.lock().unwrap().registry().register(
            listeners.get_mut(&token).unwrap(),
            token,
            Interest::READABLE,
        )?;
        Ok(token)
    }

    pub fn service_loop(&self) {
        let mut events = Events::with_capacity(1024);

        loop {
            self.poll
                .lock()
                .unwrap()
                .poll(&mut events, Some(Duration::from_secs(1))) // limit accept to 1per sec
                .unwrap();
            if log_enabled!(log::Level::Debug){
                debug!("poll timedout events.is_empty() {}", events.is_empty());
            }
            for event in events.iter() {
                let event_token = event.token();
                let mut listeners = self.listeners.lock().unwrap();
                let listener = listeners.get_mut(&event_token);
                if listener.is_some() {
                    let listener = listener.unwrap();
                    self.serice_accept(listener);
                } else {
                    info!("not listener {:?}", event_token);
                }
            }
        }
    }
    fn serice_accept(&self, listener: &mut TcpListener) {
        match listener.accept() {
            Ok((mut stream, addr)) => {
                info!("accepted connection from: {}", addr);
                let token = self.next_token();

                self.poll
                    .lock()
                    .unwrap()
                    .registry()
                    .register(&mut stream, token, Interest::READABLE)
                    .unwrap();
                let mut streams = self.streams.lock().unwrap();
                streams.insert(token, stream);
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // socket not ready anymore
                info!("accet socket error deregister: {:?}", e);

                let _ = self.poll.lock().unwrap().registry().deregister(listener);
            }
            e => {
                error!("accept error: {:?}", e);
            }
        }
    }
}

pub struct Svc<P: Messenger, C: CallbackSendRecv<P>> {
    phantom_p: std::marker::PhantomData<(P, C)>,
}
// https://github.com/tokio-rs/mio/tree/master
impl<P: Messenger, C: CallbackSendRecv<P>> Svc<P, C> {
    pub fn bind(
        addr: &str,
        callback: Arc<C>,
        name: Option<&str>,
        acceptor: SvcAcceptorRef,
    ) -> Result<(), Box<dyn Error>> {
        acceptor.register(TcpListener::bind(addr.parse()?)?);
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use std::thread::sleep;

    use super::*;
    use crate::unittest::setup::protocol::*;
    use links_network_core::prelude::*;
    use links_testing::unittest::setup;
    use log::{info, Level};

    #[test]
    fn test_svc_bind() {
        setup::log::configure();
        let addr = "0.0.0.0:8080"; // setup::net::rand_avail_addr_port();
        let clbk = LoggerCallback::<TestSvcMsgProtocol>::new_ref(Level::Info, Level::Info);
        let acceptor = SvcAcceptor::new_ref("IO");
        let res = Svc::bind(&addr, clbk, Some("test_svc_bind"), acceptor);
        info!("res: {:?}", res);
        sleep(Duration::from_secs(10));
        // assert!(res.is_ok());
        // sleep(Duration::from_secs(10));
    }

    #[test]
    fn test_channel() {
        setup::log::configure();
        let (tx, rx) = std::sync::mpsc::channel::<&u32>();
        let x = rx.try_recv();
        info!("x: {:?}", x);
        tx.send(&3).unwrap();
        let x = rx.try_recv();
        info!("x: {:?}", x);
    }
}
