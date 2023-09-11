use links_testing::unittest::setup;
use log::info;

fn main(){
    setup::log::configure();
    let p = mio::Poll::new().unwrap();
    let x = p.registry();
    
    info!("x: {:?}", x);


}