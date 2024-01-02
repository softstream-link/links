# Framer
Let's start by implementing the `Framer` trait. This trait is responsible for determining if the inbound network buffer contains sufficient number of bytes to create a complete message. In our example we will use a `delimiter` based strategy, where each message sent will be delimited by a `\n` character

You only need to implement one method here, called `get_frame_length`, which will be called every time you call `Clt`'s or `Svc`s `recv` method. The `get_frame_length` method will be passed a reference to a `bytes::BytesMut` buffer which will contain accumulated incoming network data. Application developer's job is to simply return an `Option` with position on which the frame ends or `None` when frame is incomplete.


Below example looks for the `\n` new line character and returns its positions. 

------
> **Note**: Your Protocol structure `must` also implement `Debug` & `Clone` which are typically done via `derive` macro.
>> * `Clone` is necessary because each new connection that the `Svc` accepts will get a new and independent "copy" of the `Protocol` instance. This is necessary because some methods in the `Protocol` trait require a `self` reference that enables feature of the protocol to track the state of each individual connection separately from one another.
>> * `Debug` is required to provide a meaningful messages during logging or exception handling under certain conditions.
------

```rust
# use links_nonblocking::prelude::*;
#
#[derive(Debug, Clone)] // Note that Debug & Clone are required for Protocol `struct`
struct MessageProtocol;

impl Framer for MessageProtocol {
    fn get_frame_length(bytes: &bytes::BytesMut) -> Option<usize> {
        for (idx, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                return Some(idx + 1);
            }
        }
        None
    }
}
```
