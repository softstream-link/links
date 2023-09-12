use std::{
    error::Error,
    sync::Arc,
    thread::{spawn, Builder},
    time::{Duration, Instant},
};

use links_network_core::prelude::{CallbackSendRecvNew, LoggerCallbackNew, MessengerNew, DevNullCallbackNew};
use links_network_sync::{
    prelude_nonblocking::*,
    unittest::setup::{
        framer::TEST_MSG_FRAME_SIZE,
        messenger::{TestCltMsgProtocol, TestSvcMsgProtocol},
    },
};
use links_testing::unittest::setup::{
    self,
    model::{TestCltMsg, TestCltMsgDebug},
};
use log::{info, Level};

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

    let svc = Svc::<TestSvcMsgProtocol, _, TEST_MSG_FRAME_SIZE>::bind(
        addr,
        svc_callback.clone(),
        max_connections,
        name.clone(),
    )
    .unwrap();

    info!("svc: {}", svc);

    let mut clt_initiator = Clt::<TestCltMsgProtocol, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        timeout,
        retry_after,
        clt_callback.clone(),
        name.clone(),
    )
    .unwrap();
    info!("clt_initiator: {}", clt_initiator);

    let mut clt_acceptor = svc.accept_busywait_timeout(timeout)?.unwrap();
    info!("clt_acceptor: {}", clt_acceptor);

    let mut clt_initiator_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    clt_initiator.send_busywait_timeout(&mut clt_initiator_send_msg, timeout)?;
    let clt_acceptor_recv_msg = clt_acceptor.recv_busywait_timeout(timeout)?.unwrap();

    assert_eq!(clt_initiator_send_msg, clt_acceptor_recv_msg);

    const WRITE_N_TIMES: usize = 100_000;
    let clt_acceptor_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let mut msg_recv_count = 0_usize;
            loop {
                match clt_acceptor.recv_nonblocking() {
                    Ok(ReadStatus::Completed(Some(_recv_msg))) => {
                        msg_recv_count += 1;
                    }
                    Ok(ReadStatus::Completed(None)) => {
                        info!(
                            "Connection Closed by clt_initiator clt_acceptor: {}",
                            clt_acceptor
                        );
                        break;
                    }
                    Ok(ReadStatus::WouldBlock) => continue,
                    Err(err) => {
                        panic!(
                            "Connection Closed by clt_initiator, clt_acceptor: {}, err: {}",
                            clt_acceptor, err
                        );
                    }
                }
            }
            msg_recv_count
        })
        .unwrap();

    let now = Instant::now();
    for _ in 0..WRITE_N_TIMES {
        clt_initiator.send_busywait(&mut clt_initiator_send_msg)?;
    }
    let elapsed = now.elapsed();

    drop(clt_initiator); // close the connection and allow the acceptor to exit
    let msg_recv_count = clt_acceptor_jh.join().unwrap();
    info!(
        "msg_recv_count: {}, per/write {:?}, total: {:?}",
        msg_recv_count,
        elapsed / WRITE_N_TIMES as u32,
        elapsed
    );

    Ok(())
}

fn setup<MSvc: MessengerNew, MClt: MessengerNew>() -> (
    &'static str,
    Arc<impl CallbackSendRecvNew<MSvc>>,
    Arc<impl CallbackSendRecvNew<MClt>>,
    usize,
    Option<&'static str>,
    Duration,
    Duration,
) {
    let addr = setup::net::rand_avail_addr_port();
    // let svc_callback = LoggerCallbackNew::<MSvc>::with_level_ref(Level::Debug, Level::Debug);
    // let clt_callback = LoggerCallbackNew::<MClt>::with_level_ref(Level::Debug, Level::Debug);
    let svc_callback = DevNullCallbackNew::<MSvc>::new_ref();
    let clt_callback = DevNullCallbackNew::<MClt>::new_ref();
    let name = Some("example");
    let max_connections = 0;
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