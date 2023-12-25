
# Motivation

* The motivation for the `LINKS` product is to simplify application code development where there is a need for data exchange over the network.

# How does it simplify?
* Traditionally, network api provide an interface that exposes access to generic, low level `byte arrays` where as an application layer prefers `struct`s which carry information and application state. 
* Rust's [std::net](https://doc.rust-lang.org/std/net/index.html) module is not an exception and it leaves developer with the responsibility to interpret the byte array, extract a single frame of bytes from that array, convert it into a desired data structure, while keeping track of remaining bytes and managing a lot of other details. The implementation details here have direct impact on application performance and reliability.
* Even once those details have been addressed, the developer has to solve for many additional tasks such as:
  * How to handle partial reads?
  * Can i split read and write between different threads?
  * If i do split reads into a separate thread, can i use a single thread to manage all reads?
  * ... etc, etc
  
* This library addresses above challenges, while providing a highly performant network code without imposing limitations on application api options.
  
# Please tell me more
* At a very high level The main concept is based on the the following two `struct`ures
  * `Clt` - this is a network client and can initiate a connection
  * `Svc` - this is a network service which listens to a port and creates a `Clt` for each established connection
  * Both `Clt` and `Svc` then provide and `send` and `recv` method with a signature that roughly looks like this:
    * `Clt::send(&msg: T)` vs `Clt::recv() -> T` - where `T` is a generic type that you specify when instantiating a `Clt` and `Svc`
  
* There are three implementations available.
  * [links_nonblocking](./nonblocking/readme.md) - this implementation is most complete at the moment and its `send()`/`recv()`  methods take a `timeout` argument. This allows the application developer to set `io` wait limits. The internal implementation relies on `spin` locks and waits to provide `best` latency performance as it does not let `OS` to park the running thread, which incurs significant latency penalty. This implementation is recommended for cases with `low latency` performance requirement.
  * [links_blocking](./blocking/readme.md) - this implementation is simplest to use as 
  all method calls look sequential and block until the operation completes. The ease of use comes at the cost of performance and scalability. This implementation is recommended for your typical network loads.
  * links_async - this implementation is based on Rust's `async`/`await` `tokio` framework, however, at the moment of this writing Rust's async api is still going through stabilization and is not yet available on `stable`.
