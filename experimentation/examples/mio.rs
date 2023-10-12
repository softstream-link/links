use std::fmt::{Debug, };

use links_core::unittest::setup;
use log::info;

#[derive(Debug, Clone)]
struct One<T: Debug> {
    inner: T,
}
impl<T: Debug> NewTrait for One<T> {}
#[derive(Debug, Clone)]
struct Two {}
impl NewTrait for Two {}

#[derive(Debug, Clone)]
struct Container<T: Clone+Debug> {
    inner: T,
}
impl<T: Clone+Debug> NewTrait for Container<T> {}

trait NewTrait: Debug {}

fn main() {
    setup::log::configure();
    let one = Container { inner: One { inner: 2} };
    let two = Container { inner: Two {} };
    info!("one: {:?}", one);
    info!("two: {:?}", two);

    let mut slab = slab::Slab::<Box<dyn NewTrait>>::new();
    slab.insert(Box::new(one));
    slab.insert(Box::new(two));
}

// fn main() {
//     setup::log::configure();
//     let one = Container { inner: One {} };
//     let two = Container { inner: Two {} };
//     info!("one: {:?}", one);
//     info!("two: {:?}", two);

//     let mut slab = slab::Slab::new();
//     slab.insert(Box::new(one));
//     slab.insert(Box::new(two));
// }
