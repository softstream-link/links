use links_core::unittest::setup;
use log::info;

fn main() {
    setup::log::configure();
    let p = mio::Poll::new().unwrap();
    let x = p.registry();

    info!("x: {:?}", x);
}
