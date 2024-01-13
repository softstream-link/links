pub mod conid;
pub mod counters;
pub mod framer;
pub mod macros;
pub mod messenger;
pub mod pool;

/// Trait defining a shutdown hook for a connection
pub trait Shutdown {
    fn shutdown(&mut self);
}
