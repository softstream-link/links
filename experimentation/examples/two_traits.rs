use slab::Slab;

trait DoerTrait {
    fn next(&mut self);
}
trait FactoryTrait: DoerTrait {
    type D: DoerTrait;
    fn create(&mut self) -> Self::D;
}
////
struct CreatorImpl<const SOME_GENERIC: usize>;
impl<const SOME_GENERIC: usize> DoerTrait for CreatorImpl<SOME_GENERIC> {
    fn next(&mut self) {
        println!("StaticImpl::next");
    }
}
impl<const SOME_GENERIC: usize> FactoryTrait for CreatorImpl<SOME_GENERIC> {
    type D = DoerImpl<SOME_GENERIC>;
    fn create(&mut self) -> Self::D {
        DoerImpl::<SOME_GENERIC> {}
    }
}
////
struct DoerImpl<const SOME_GENERIC: usize>;
impl<const SOME_GENERIC: usize> DoerTrait for DoerImpl<SOME_GENERIC> {
    fn next(&mut self) {
        println!("DoerImpl::next");
    }
}
struct CreatorDyn(Box<dyn FactoryTrait<D=DoerDyn>>);
struct DoerDyn(Box<dyn DoerTrait>);

enum Doable<F: FactoryTrait> {
    Doer(F::D),
    Creator(F),
}
struct CreatorHandler<F: FactoryTrait> {
    slap: Slab<Doable<F>>,
}
impl<F: FactoryTrait> CreatorHandler<F> {
    pub fn new() -> Self {
        Self {
            slap: Slab::<Doable<F>>::new(),
        }
    }
    fn add(&mut self, creator: F) {
        let key = self.slap.insert(Doable::Creator(creator));
    }
    fn run(&mut self, key: usize) {
        match self.slap[key] {
            Doable::Doer(ref mut doer) => doer.next(),
            Doable::Creator(ref mut creator) => {
                let doer = creator.create();
                let _key = self.slap.insert(Doable::Doer(doer));
            }
        }
    }
}

fn main() {
    let mut slap = Slab::<CreatorImpl<3>>::new();

    let a = slap.insert(CreatorImpl::<3> {});
    // let a = slap.insert(StaticImpl::<2>{}); // FAILS because of generic, as expected
    // let slap = Slab::<StaticImpl<4>>::new();
}
