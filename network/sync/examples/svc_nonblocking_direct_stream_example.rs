use std::{sync::Arc, time::Duration};

use links_network_core::prelude::{CallbackRecv, CallbackSend, LoggerCallbackNew, MessengerNew};
use links_network_sync::{
    connect::{clt::nonblocking::Clt, svc::nonblocking::Svc},
    unittest::setup::{
        framer::TEST_MSG_FRAME_SIZE,
        messenger::{TestCltMsgProtocol, TestSvcMsgProtocol},
    },
};
use links_testing::unittest::setup;
use log::info;

fn main() {
    setup::log::configure();
    let (
        addr,
        svc_callback_recv,
        svc_callback_send,
        clt_callback_recv,
        clt_callback_send,
        max_connections,
        name,
        timeout,
        retry_after,
    ) = setup();


    let svc = Svc::<TestSvcMsgProtocol, _, _, TEST_MSG_FRAME_SIZE>::bind(
        addr,
        svc_callback_recv.clone(),
        svc_callback_send.clone(),
        max_connections,
        name.clone(),
    )
    .unwrap();

    info!("svc: {}", svc);

    let clt_initiator = Clt::<TestCltMsgProtocol, _, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        timeout,
        retry_after,
        clt_callback_recv,
        clt_callback_send,
        name,
    )
    .unwrap();
    info!("clt_initiator: {}", clt_initiator);

    let clt_acceptor = svc.accept_busy_wait(timeout).unwrap();
    info!("clt_acceptor: {}", clt_acceptor);


}




fn setup<MSvc: MessengerNew, MClt: MessengerNew>() -> (
    &'static str,
    Arc<impl CallbackRecv<MSvc>>,
    Arc<impl CallbackSend<MSvc>>,
    Arc<impl CallbackRecv<MClt>>,
    Arc<impl CallbackSend<MClt>>,
    usize,
    Option<&'static str>,
    Duration,
    Duration,
) {
    let addr = setup::net::rand_avail_addr_port();
    let svc_clbk_recv = LoggerCallbackNew::<MSvc>::new_ref();
    let svc_clbk_send = LoggerCallbackNew::<MSvc>::new_ref();
    let clt_clbk_recv = LoggerCallbackNew::<MClt>::new_ref();
    let clt_clbk_send = LoggerCallbackNew::<MClt>::new_ref();
    let name = Some("example");
    let max_connections = 2;
    let timeout = Duration::from_micros(1_000);
    let retry_after = Duration::from_micros(100);
    (
        addr,
        svc_clbk_recv,
        svc_clbk_send,
        clt_clbk_recv,
        clt_clbk_send,
        max_connections,
        name,
        timeout,
        retry_after,
    )
}
