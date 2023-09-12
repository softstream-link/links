use std::{error::Error, sync::Arc, time::Duration};

use links_network_core::prelude::{CallbackSendRecvNew, LoggerCallbackNew, MessengerNew};
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
use log::info;

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
#[test]
fn test() -> Result<(), Box<dyn Error>> {
    run()
}
fn run() -> Result<(), Box<dyn Error>> {
    setup::log::configure();
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

    let mut clt_acceptor = svc.accept_busywait(timeout).unwrap();
    info!("clt_acceptor: {}", clt_acceptor);

    let mut clt_initiator_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));
    clt_initiator.send_busywait(&mut clt_initiator_send_msg)?;
    let clt_acceptor_recv_msg = clt_acceptor.recv_busywait()?.unwrap();

    assert_eq!(clt_initiator_send_msg, clt_acceptor_recv_msg);

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
    let svc_callback = LoggerCallbackNew::<MSvc>::new_ref();
    let clt_callback = LoggerCallbackNew::<MClt>::new_ref();
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
