use std::{error::Error, num::NonZeroUsize, time::Instant};

use links_core::{
    fmt_num,
    prelude::DevNullCallback,
    unittest::setup::{
        self,
        framer::TEST_MSG_FRAME_SIZE,
        messenger::{CltTestMessenger, SvcTestMessenger},
        model::*,
    },
};
use links_nonblocking::prelude::*;
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

    let addr = setup::net::rand_avail_addr_port();

    let store = CanonicalEntryStore::<TestMsg>::new_ref();
    // let clbk = StoreCallback::<SvcTestMessenger, _, _>::new_ref(store.clone());

    let svc = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(addr, StoreCallback::new_ref(store.clone()), NonZeroUsize::new(1).unwrap(), Some("example/svc")).unwrap();
    info!("svc: {}", svc);

    let (svc_acceptor, _, mut svc_sender) = svc.into_split();
    let mut poll_handler = PollHandlerDynamic::default();
    poll_handler.add_acceptor(svc_acceptor.into());
    let spawned_poll_handler = poll_handler.into_spawned_handler("Svc-Poll-Thread");

    let clt = Clt::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        DevNullCallback::new_ref(),
        Some("example/clt"),
    )
    .unwrap();
    info!("clt: {}", clt);
    let (clt_recver, mut clt_sender) = clt.into_split();
    spawned_poll_handler.add_recver(clt_recver.into());

    let mut clt_msgs = vec![
        TestCltMsg::Login(TestCltMsgLoginReq::default()),
        TestCltMsg::HBeat(TestHBeatMsgDebug::default()),
        TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg")),
    ];
    let mut svc_msgs = vec![
        TestSvcMsg::Accept(TestSvcMsgLoginAcpt::default()),
        TestSvcMsg::HBeat(TestHBeatMsgDebug::default()),
        TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Server Msg")),
    ];

    info!("clt_sender: {}", clt_sender);

    let now = Instant::now();
    for msg in clt_msgs.iter_mut() {
        clt_sender.send_busywait(msg)?;
    }

    svc_sender.pool_accept_busywait()?; // ensure there there is sufficient time for poll_handler thread to wake up and accept incoming connection
    for msg in svc_msgs.iter_mut() {
        svc_sender.send_busywait(msg)?;
    }

    let elapsed = now.elapsed();

    drop(clt_sender);

    // VERIFY numbers of messages sent and received
    info!(
        "msg_send_count: {}, per/write {:?}, total: {:?}",
        fmt_num!(clt_msgs.len() + svc_msgs.len()),
        elapsed / clt_msgs.len() as u32,
        elapsed
    );

    let found = store
        .find_recv(
            "example/svc",
            |msg| matches!(msg, TestMsg::Clt(TestCltMsg::Dbg(TestCltMsgDebug{text, ..})) if text == &b"Hello Frm Client Msg".as_slice().into()),
            setup::net::optional_find_timeout(),
        )
        .unwrap();

    info!("found: {:?}", found);
    assert_eq!(found.try_into_clt(), clt_msgs[2]);

    info!("store: {}", store);
    assert_eq!(store.len(), clt_msgs.len() + clt_msgs.len());

    Ok(())
}