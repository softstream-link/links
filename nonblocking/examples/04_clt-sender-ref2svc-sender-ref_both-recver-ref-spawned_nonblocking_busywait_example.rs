use links_core::{
    prelude::DevNullCallback,
    unittest::setup::{self, framer::TEST_MSG_FRAME_SIZE, model::*},
};
use links_nonblocking::{
    prelude::*,
    unittest::setup::protocol::{CltTestProtocolAuthAndHbeat, SvcTestProtocolAuthAndHBeat},
};
use log::info;
use std::{error::Error, num::NonZeroUsize, thread::sleep};

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

    let store = CanonicalEntryStore::<UniTestMsg>::new_ref();

    let protocol = SvcTestProtocolAuthAndHBeat::default();
    let name = Some("example/svc");
    let mut svc_sender = Svc::<_, _, TEST_MSG_FRAME_SIZE>::bind(addr, NonZeroUsize::new(1).unwrap(), StoreCallback::new_ref(store.clone()), protocol, name)
        .unwrap()
        .into_sender_with_spawned_recver_ref();
    info!("svc_sender: {}", svc_sender);

    let protocol = CltTestProtocolAuthAndHbeat::default();
    let interval = protocol.conf_heart_beat_interval().unwrap();
    let name = Some("example/clt");
    let mut clt_sender = Clt::<_, _, TEST_MSG_FRAME_SIZE>::connect(addr, setup::net::default_connect_timeout(), setup::net::default_connect_retry_after(), DevNullCallback::new_ref(), protocol, name)
        .unwrap()
        .into_sender_with_spawned_recver_ref();
    info!("clt_sender: {}", clt_sender);

    let mut clt_msgs = vec![CltTestMsgDebug::new(b"Hello Frm Client Msg").into(), CltTestMsgPing::default().into()];
    let mut svc_msgs = vec![SvcTestMsgDebug::new(b"Hello Frm Server Msg").into()]; // SvcTestProtocolAuthAndHBeat should send_reply with Pong to Ping

    for msg in clt_msgs.iter_mut() {
        clt_sender.send_busywait(msg)?;
    }

    // svc_sender.accept_into_pool_busywait()?; // ensure there there is sufficient time for poll_handler thread to wake up and accept incoming connection
    for msg in svc_msgs.iter_mut() {
        svc_sender.send_busywait(msg)?;
    }

    let allow_n_hbeats = 2_usize;
    sleep(interval * allow_n_hbeats as u32 - (interval / 2)); // less half interval to ensure that no n + 1 hbeats are sent
    drop(clt_sender);

    // VERIFY numbers of messages sent and received

    info!("store: {}", store);
    let expected_msg_count = clt_msgs.len() + svc_msgs.len() + 2 + allow_n_hbeats * 2 + 1; // 2 is from the auth handshake , * 2 of hbeats for clt and svc, + 1 for svc pong reply
    assert_eq!(store.len(), expected_msg_count);

    // find debug from clt
    let found = store.find_recv(
        "example/svc",
        |msg| matches!(msg, UniTestMsg::Clt(CltTestMsg::Dbg(CltTestMsgDebug{text, ..})) if text == &b"Hello Frm Client Msg".as_slice().into()),
        setup::net::optional_find_timeout(),
    );
    info!("found: {:?}", found);
    assert_eq!(found.unwrap().try_into_clt(), CltTestMsg::Dbg(CltTestMsgDebug::new(b"Hello Frm Client Msg")));

    // find hbeat from clt
    let found = store.find_recv("example/svc", |msg| matches!(msg, UniTestMsg::Clt(CltTestMsg::HBeat(_))), setup::net::optional_find_timeout());
    info!("found: {:?}", found);
    assert_eq!(found.unwrap().try_into_clt(), CltTestMsg::HBeat(UniTestHBeatMsgDebug::default()));

    // find sent reply from svc
    let found = store.find_sent("example/svc", |msg| matches!(msg, UniTestMsg::Svc(SvcTestMsg::Pong(_))), setup::net::optional_find_timeout());
    info!("found: {:?}", found);
    assert_eq!(found.unwrap().try_into_svc(), SvcTestMsg::Pong(SvcTestMsgPong::default()));
    Ok(())
}
