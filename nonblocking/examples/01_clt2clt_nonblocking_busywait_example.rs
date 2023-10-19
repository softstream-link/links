use std::{error::Error, num::NonZeroUsize, thread::Builder, time::Instant};

use links_core::{
    fmt_num,
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
    const WRITE_N_TIMES: usize = 100_000;

    let svc_jh = Builder::new()
        .name("Acceptor-Thread".to_owned())
        .spawn(move || {
            let svc = Svc::<SvcTestMessenger, _, TEST_MSG_FRAME_SIZE>::bind(addr, DevNullCallback::<SvcTestMessenger>::new_ref(), NonZeroUsize::new(1).unwrap(), Some("example/clt")).unwrap();

            info!("svc: {}", svc);
            let mut clt = svc.accept_busywait_timeout(setup::net::default_connect_timeout()).unwrap().unwrap();
            info!("clt: {}", clt);

            let mut clt_send_msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"Hello Frm Client Msg"));
            let mut msg_recv_count = 0_usize;
            loop {
                if let Ok(Some(_msg)) = clt.recv_busywait() {
                    msg_recv_count += 1;
                    clt.send_busywait(&mut clt_send_msg).unwrap();
                    continue;
                } else {
                    break;
                }
            }
            msg_recv_count
        })
        .unwrap();

    let mut clt = Clt::<CltTestMessenger, _, TEST_MSG_FRAME_SIZE>::connect(
        addr,
        setup::net::default_connect_timeout(),
        setup::net::default_connect_retry_after(),
        DevNullCallback::<CltTestMessenger>::new_ref(),
        Some("example/svc"),
    )
    .unwrap();
    info!("clt {}", clt);

    let mut clt_send_msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"Hello Frm Client Msg"));

    // send the first message to the server to establish connection
    clt.send_busywait_timeout(&mut clt_send_msg, setup::net::default_connect_timeout())?;
    let now = Instant::now();
    for _ in 0..WRITE_N_TIMES {
        clt.send_busywait(&mut clt_send_msg)?;
        let _msg = clt.recv_busywait().unwrap().unwrap();
    }
    let elapsed = now.elapsed();

    drop(clt); // close the connection and allow the acceptor to exit
    let msg_recv_count = svc_jh.join().unwrap();
    info!(
        "msg_send_count: {}, msg_recv_count: {}, per/write {:?}, total: {:?}",
        fmt_num!(WRITE_N_TIMES),
        fmt_num!(msg_recv_count),
        elapsed / WRITE_N_TIMES as u32,
        elapsed
    );
    assert_eq!(msg_recv_count, WRITE_N_TIMES + 1); // +1 for the first message to connect
    Ok(())
}
