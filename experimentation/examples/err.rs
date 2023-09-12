use std::{io::ErrorKind, fmt::Display};

use links_testing::unittest::setup;
use log::info;

use std::error::Error;

#[derive(Debug)]
struct MyError{
    msg: String,
}
impl Display for MyError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyError: {}", self.msg)
    }
}
impl Error for MyError {}

fn main(){
    setup::log::configure();
    let e = std::io::Error::new(ErrorKind::Other, "test");
    
    // let e = std::io::Error::new(ErrorKind::, "test");
    info!("Error: {:?}", e);

    let e2 = MyError{msg: "test".to_string()};
    info!("Error: {:?}", e2);
    let e3 = std::io::Error::new(ErrorKind::Other, e2);
    info!("Error: {:?}", e3);
    // let e4 = e3.source();
    // info!("Error: {:?}", e4);
    let e4= e3.into_inner();
    info!("Error: {:?}", e4.unwrap());

    let e3 = std::io::Error::new(ErrorKind::TimedOut, "blahs");
    info!("Error: {:?}", e3);

    // let e5: std::io::Error = e2.into();
    // info!("Error: {:?}", e5);


}