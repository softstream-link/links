# Messenger

Now lets implement the `Messenger` trait for our `Clt` & `Svc` types. The `Messenger` trait is responsible for serializing & deserializing messages into bytes. It also specifies what message types `Clt` & `Svc` will be able to send & receive.

A few things to note about the `Messenger` trait implementation:
1. The two associated types `RecvT` & `SendT` are used to specify what message types `Clt` & `Svc` will be able to `recv` & `send`. In our example, we chose to use the same message type for both `Clt` & `Svc` but in a real world scenario `Clt` & `Svc` would likely have different message types. Hence, both `Clt` & `Svc` would need to provide their own implementation of the `Messenger` trait. That would mean there will be two separate structures `CltMessageProtocol` & `SvcMessageProtocol` one for `Clt` & one for `Svc` respectively.
   
2. `links` library is designed with performance in mind and is aiming to avoid runtime heap allocation. As a result `Messenger::deserialize` method signature returns an owned type instead of a smart pointer, while `Messenger::serialize` returns a fixed size `stack allocated` byte array. Note that to a void a stack frame copy on these function invocations we encourage you to `inline` both of these method's implementations.
   
   > Note: The `Messenger::serialize<const MAX_MSG_SIZE: usize>` has a generic `const` argument that will be propagated from instantiations that will looks something like this:
   > * `Clt::<_, _, MAX_MSG_SIZE>::connect(...)` 
   > * `Svc::<_, _, MAX_MSG_SIZE>::bind(...)` 
   
   It is also important to note that in our example we choose to deserialize a byte array into a json `String`, which requires heap allocation, before converting it into a `ExchangeDataModel` enum. This is done for simplicity of the example, but in a real world you would likely choose to deserialize without a json `String` step and avoid `heap allocation` by going direction from the byte array into a `ExchangeDataModel`. One of the ways of doing it is by leveraging [byteserde](https://crates.io/crates/byteserde) crate instead of [serde_json](https://crates.io/crates/serde_json).


3. `ProtocolCore` & `Protocol` traits provide advanced features, that will not be covered in this example but need to be implemented anyway. The default implementations are sufficient and will be optimized away by the compiler unless overridden.
 
```rust
# use links_nonblocking::prelude::*;
#
# #[derive(Debug, serde::Serialize, serde::Deserialize)]
# struct Ping;
# #[derive(Debug, serde::Serialize, serde::Deserialize)]
# struct Pong;
# 
# #[derive(Debug, serde::Serialize, serde::Deserialize)]
# enum ExchangeDataModel {
#     Ping(Ping),
#     Pong(Pong),
# }
# 
# #[derive(Debug, Clone)] // Note Clone required for Protocol
# struct MessageProtocol;
# 
# impl Framer for MessageProtocol {
#     fn get_frame_length(bytes: &bytes::BytesMut) -> Option<usize> {
#         for (idx, byte) in bytes.iter().enumerate() {
#             if *byte == b'\n' {
#                 return Some(idx + 1);
#             }
#         }
#         None
#     }
# }
#
impl Messenger for MessageProtocol {
    type RecvT = ExchangeDataModel;
    type SendT = ExchangeDataModel;
    #[inline(always)] // DO inline to avoid a potentially expensive stack frame copy
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, std::io::Error> {
        let frame = std::str::from_utf8(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let received = serde_json::from_str(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(received)
    }
    #[inline(always)] // DO inline to avoid a potentially expensive stack frame copy
    fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), std::io::Error> {
        let msg = serde_json::to_string(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut msg = msg.into_bytes();
        msg.push(b'\n');
        let mut buf = [0_u8; MAX_MSG_SIZE];
        buf[..msg.len()].copy_from_slice(&msg);
        Ok((buf, msg.len()))
    }
}

// Default implementation is sufficient for this example and will be optimized away by the compiler
impl ProtocolCore for MessageProtocol {} 
impl Protocol for MessageProtocol {}
```