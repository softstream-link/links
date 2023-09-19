use std::{
    error::Error,
    io::ErrorKind,
    sync::Arc,
    thread::Builder,
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackRecvSend, DevNullCallback, Messenger};
use links_network_sync::{
    prelude_nonblocking::*,
    unittest::setup::{
        framer::TEST_MSG_FRAME_SIZE,
        messenger::{TestCltMsgProtocol, TestSvcMsgProtocol},
    },
};
use links_testing::unittest::setup::{
    self,
    model::{TestCltMsg, TestCltMsgDebug, TestSvcMsg, TestSvcMsgDebug},
};
use log::info;

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
#[test]
fn test() -> Result<(), Box<dyn Error>> {
    run()
}
fn run() -> Result<(), Box<dyn Error>> {
    setup::log::configure_level(log::LevelFilter::Info);
    let (addr, svc_callback, clt_callback, max_connections, name, timeout, retry_after) = setup();

    let mut svc = Svc::<TestSvcMsgProtocol, _, TEST_MSG_FRAME_SIZE>::bind(
        addr,
        svc_callback.clone(),
        max_connections,
        name.clone(),
    )
    .unwrap();

    info!("svc: {}", svc);

    let mut clt = Clt::<TestCltMsgProtocol, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        timeout,
        retry_after,
        clt_callback.clone(),
        name.clone(),
    )
    .unwrap();
    info!("clt: {}", clt);

    svc.pool_accept_busywait_timeout(timeout)?.unwrap_accepted();

    info!("svc: {}", svc);

    let mut clt_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    clt.send_busywait_timeout(&mut clt_send_msg, timeout)?;
    let svc_recv_msg = svc.recv_busywait()?.unwrap();

    assert_eq!(clt_send_msg, svc_recv_msg);

    const WRITE_N_TIMES: usize = 100_000;
    let svc_jh = Builder::new()
        .name("Svc-Thread".to_owned())
        .spawn(move || {
            let mut svc_send_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Client Msg"));
            let mut msg_recv_count = 0_usize;

            while let Ok(Some(_msg)) = svc.recv_busywait() {
                msg_recv_count += 1;
                svc.send_busywait(&mut svc_send_msg).unwrap();
            }
            (msg_recv_count, svc)
        })
        .unwrap();

    let now = Instant::now();
    for _ in 0..WRITE_N_TIMES {
        clt.send_busywait(&mut clt_send_msg)?;
        let _msg = clt.recv_busywait().unwrap().unwrap();
    }
    let elapsed = now.elapsed();

    drop(clt); // close the connection and allow the acceptor to exit

    // VERIFY numbers of messages sent and received
    let (msg_recv_count, mut svc) = svc_jh.join().unwrap();
    info!(
        "msg_send_count: {}, msg_recv_count: {} , per/write {:?}, total: {:?}",
        WRITE_N_TIMES.to_formatted_string(&Locale::en),
        msg_recv_count.to_formatted_string(&Locale::en),
        elapsed / WRITE_N_TIMES as u32,
        elapsed
    );
    assert_eq!(msg_recv_count, WRITE_N_TIMES);

    // VERIFY svc interl pool returns None to all calls.
    let svc_recv_err = svc.recv_busywait().unwrap_err();
    info!("svc_recv_err: {}", svc_recv_err);
    assert_eq!(svc.pool_recv_send_len(), (0, 1)); // 0 recv, 1 send in pool recver was dropped in the Svc-Thread
    assert_eq!(svc_recv_err.kind(), ErrorKind::NotConnected); // if there are no recives pool returns NotConnected

    let mut svc_send_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Client Msg"));

    let svc_send_err = svc.send_busywait(&mut svc_send_msg).unwrap_err();
    info!("svc_send_err: {}", svc_send_err);
    assert_eq!(svc.pool_recv_send_len(), (0, 0)); // last sender was dropped after attemp to send_busywait
    assert_eq!(svc_send_err.kind(), ErrorKind::BrokenPipe); // because send_busywait tried to use the last sender in the pool it errored out with BrokenPipe

    Ok(())
}

fn setup<MSvc: Messenger, MClt: Messenger>() -> (
    &'static str,
    Arc<impl CallbackRecvSend<MSvc>>,
    Arc<impl CallbackRecvSend<MClt>>,
    usize,
    Option<&'static str>,
    Duration,
    Duration,
) {
    let addr = setup::net::rand_avail_addr_port();
    // let svc_callback = LoggerCallbackNew::<MSvc>::with_level_ref(Level::Debug, Level::Debug);
    // let clt_callback = LoggerCallbackNew::<MClt>::with_level_ref(Level::Debug, Level::Debug);
    let svc_callback = DevNullCallback::<MSvc>::new_ref();
    let clt_callback = DevNullCallback::<MClt>::new_ref();
    let name = Some("example");
    let max_connections = 1; // TODO make this NonZeroUsize as CicleIterator fails if if max_connections is 0 of fix CicleIterator
    let timeout = Duration::from_micros(1_000);
    let retry_after = Duration::from_micros(100);
    (
        addr,
        svc_callback,
        clt_callback,
        max_connections,
        name,
        timeout,
        retry_after,
    )
}
