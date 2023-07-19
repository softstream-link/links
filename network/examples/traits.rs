trait Callback{
    fn on_recv(&self, msg: &[u8]);
}
struct LoggerCallback;

impl Callback for LoggerCallback{
    fn on_recv(&self, msg: &[u8]){
        println!("LoggerCallback: {:?}", msg);
    }
}

struct Clt2;
impl Clt2{
    fn new(callback: impl Callback) -> Self{  // --> WORKS
        println!("Clt2::new");
        Self{}
    }
}

struct Clt{
    callback: impl Callback, // --> DOES NOT WORK
}
impl Clt{
    fn new(callback: impl Callback) -> Self{
        Self{ callback }
    }
    fn send(&self, msg: &[u8]){
        self.callback.on_recv(msg);
    }
}

struct Logger{

}
impl Logger{
    fn new() -> Self{
        Self{}
    }
}
impl Callback for Logger{
    fn on_recv(&self, msg: &[u8]){
        println!("Logger: {:?}", msg);
    }
}

struct Cacher<'a>{
    store: Vec<&'a [u8]>
}

impl<'a> Cacher<'a>{
    fn new() -> Self{
        Self{ store : Vec::new()}
    }
}
impl<'a> Callback for Cacher<'a>{
    fn on_recv(&self, msg: &[u8]){
        self.store.push(msg);
    }
}

fn main(){
    let logger = Logger::new();
    let clt = Clt::new(logger);
    clt.send(&b"to the log"[..]);

    // vs

    let cacher = Cacher::new();
    let clt = Clt::new(cacher);
    clt.send(&b"to the store"[..]);

    println!("Hello, world!");
}