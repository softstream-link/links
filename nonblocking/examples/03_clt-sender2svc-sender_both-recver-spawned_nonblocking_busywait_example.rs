use std::{error::Error, num::NonZeroUsize};

use links_core::{
    prelude::DevNullCallback,
    unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE, model::*},
};
use links_nonblocking::{
    prelude::*,
    unittest::setup::protocol::{CltTestProtocolAuthAndHbeat, SvcTestProtocolAuthAndHBeat},
};
use log::info;

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
#[test]
fn test_03() -> Result<(), Box<dyn Error>> {
    run()
}
fn run() -> Result<(), Box<dyn Error>> {
    setup::log::configure_level(log::LevelFilter::Info);

    let addr = setup::net::rand_avail_addr_port();

    let store = CanonicalEntryStore::<UniTestMsg>::new_ref();

    let protocol = SvcTestProtocolAuthAndHBeat::default(); // ensures on_connect is called
    let name = Some("example/svc");
    let mut svc_sender = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), protocol, name)
        .unwrap()
        .into_sender_with_spawned_recver();
    info!("svc_sender: {}", svc_sender);

    let protocol = CltTestProtocolAuthAndHbeat::default(); // ensures on_connect is called
    let name = Some("example/clt");
    let mut clt_sender = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), DevNullCallback::new_ref(), protocol, name)
        .unwrap()
        .into_sender_with_spawned_recver();
    info!("clt_sender: {}", clt_sender);

    let mut clt_msgs = vec![UniTestHBeatMsgDebug::default().into(), CltTestMsgDebug::new(b"Hello Frm Client Msg").into()];
    let mut svc_msgs = vec![UniTestHBeatMsgDebug::default().into(), SvcTestMsgDebug::new(b"Hello Frm Server Msg").into()];

    for msg in clt_msgs.iter_mut() {
        clt_sender.send_busywait(msg)?;
    }

    for msg in svc_msgs.iter_mut() {
        svc_sender.send_busywait(msg)?;
    }

    // info!("store: {}", store);
    // do find before printing store as find has a build in timeout and allows all messages to be received
    // asserting store.len() before find will sometimes fail if poll thread did not have enough time to wake up to process all messages
    let found = store.find_recv(
        "example/svc",
        |msg| matches!(msg, UniTestMsg::Clt(CltTestMsg::Dbg(CltTestMsgDebug{text, ..})) if text == &b"Hello Frm Client Msg".as_slice().into()),
        setup::net::optional_find_timeout(),
    );

    info!("found: {:?}", found);
    assert_eq!(found.unwrap().try_into_clt(), CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg")));

    // VERIFY numbers of messages sent and received
    info!("store: {}", store);
    let expected_msg_count = clt_msgs.len() + svc_msgs.len() + 2; // recv clt + sent svc + 2(recv/sent) is from the auth handshake
    assert_eq!(store.len(), expected_msg_count);

    Ok(())
}
