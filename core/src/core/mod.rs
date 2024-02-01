pub mod conid;
pub mod counters;
pub mod framer;
pub mod macros;
pub mod messenger;
pub mod pool;

/// Trait defining a shutdown hook for a connection
pub trait Shutdown {
    /// Typically you should not call this method directly when using `Rust` and instead rely on dropping the object which will in 
    /// turn call this method. This hook exists to provide `python` extensions with a way to shutdown the connection using a context manager
    /// or similar construct and not to rely on the garbage collector to issue a drop when the last reference remains
    fn __exit__(&mut self);
}
