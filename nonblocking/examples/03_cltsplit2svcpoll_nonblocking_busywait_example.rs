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
    const WRITE_N_TIMES: usize = 3;

    let svc = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(
        addr,
        LoggerCallback::<SvcTestMessenger>::new_ref(),
        NonZeroUsize::new(1).unwrap(),
        Some("example/svc"),
    )
    .unwrap();
    info!("svc: {}", svc);

    let (pool_acceptor, _, _pool_svc_sender) = svc.into_split();
    let mut poll_handler = PollHandlerStatic::default();
    poll_handler.add(pool_acceptor).unwrap();
    poll_handler.spawn("Svc-Poll-Thread");

    let clt = Clt::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        DevNullCallback::<CltTestMessenger>::new_ref(),
        Some("example/clt"),
    )
    .unwrap();
    info!("clt: {}", clt);
    let (_clt_recver, mut clt_sender) = clt.into_split();

    let mut clt_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));

    info!("clt_sender: {}", clt_sender);

    let now = Instant::now();
    for _ in 0..WRITE_N_TIMES {
        clt_sender.send_busywait(&mut clt_send_msg)?;
        // let _msg = clt_recver.recv_busywait().unwrap().unwrap();
    }
    let elapsed = now.elapsed();

    drop(clt_sender);

    // VERIFY numbers of messages sent and received
    let msg_recv_count = 0; //svc_jh.join().unwrap();
    info!(
        "msg_send_count: {}, msg_recv_count: {} , per/write {:?}, total: {:?}",
        fmt_num!(WRITE_N_TIMES),
        fmt_num!(msg_recv_count),
        elapsed / WRITE_N_TIMES as u32,
        elapsed
    );
    // TODO complete assert once a Arc callback is removed
    // assert_eq!(msg_recv_count, WRITE_N_TIMES + 1); // +1 for the fist message to connect


    Ok(())
}
