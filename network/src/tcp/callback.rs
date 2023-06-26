use log::info;


pub trait ReadCallback: Send {
    fn on_read(&self, bytes: &[u8]);
}


pub(crate) struct CircularBuffer<const N: usize> {
    buffer: [u8; N],
    read_index: usize,
    write_index: usize,
}

impl<const N: usize> ReadCallback for CircularBuffer<N> {
    fn on_read(&self, bytes: &[u8]) {
        println!("on_read: {:?}", bytes);
        // for byte in bytes {
        //     self.write(*byte);
        // }
    } 
}

#[derive(Debug)]
pub struct LoggerCallback;
impl ReadCallback for LoggerCallback {
    fn on_read(&self, bytes: &[u8]) {
        info!("on_read: {:?}", bytes);
    }
}
