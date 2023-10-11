use slab::Slab;

trait NonBlockingIoFn: Send+Sync+'static {
    fn execute(&self) -> bool;
}
pub struct TypeOne {}
impl NonBlockingIoFn for TypeOne {
    fn execute(&self) -> bool {
        true
    }
}
pub struct TypeTwo {}
impl NonBlockingIoFn for TypeTwo {
    fn execute(&self) -> bool {
        true
    }
}
pub fn main() {
    let mut col = Slab::<Box<dyn NonBlockingIoFn>>::new();
    // col.insert(Box::new(TypeOne {}));
}
