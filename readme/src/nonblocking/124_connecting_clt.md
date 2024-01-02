# Connecting Clt

The last step remaining is to connect the `Clt` to a running `Svc`. Below is a complete example of how to do that. Lets just review some of the `Clt` parameters below.

1. `addr` - ip/port of the `Svc` that `Clt` will connect to, since we are running both `Clt` & `Svc` on the same machine we will use same address.

2. `timeout`/`retry_after` - this is a timeout that `Clt` will use when attempting to connect to `Svc`. On each failed attempt the `Clt` will sleep for duration of `retry_after` period. Eventually if `Clt` is unable to establish a connection with in the `timeout` and error will be returned.

3. `callback` - just like with the `Svc` we will use `LoggerCallback` to log all messages `received` & `sent` by `Clt`.
   > Note: the API requires an `Arc` reference to the callback, because typically one would not just want to log messages but also do something with them. In order to achieve this the application developer needs to keep a reference to the callback for themselves and pass a `Arc::clone` of this reference to the api. 

4. `protocol` - this is an instance of the protocol that we created earlier
   > Note: we are using the same protocol `type` & `instance` for both `Clt` & `Svc` for simplicity of the example. However, in a real world scenario `Clt` & `Svc` would likely have different types as they would likely only be able to receive message types which the other can send and visa-versa.

5. `name` - this is an optional name that will be used by `LoggerCallback` to identify `Clt` instance in the logs.
   
6. `MAX_MSG_SIZE` - this determines the size of the maximum size byte array you can allocate in the `Messenger::serialize` method call

7. `into_sender_with_spawned_recver` - another important details is that we called this method for both `Clt` & `Svc` after establishing a `connect` & `bind` respectively. What this does is it internally `splits` the connection into a `sender` & `receiver` and only returns the `sender` from the api, while at the same time a new thread will be spawned to manage `receiver` side of the connection and and all of the messages will be pushed into the `callback` struct that we provided.

8. `send_busywait_timeout` - now both `Clt` to `Svc` can use a `send_busywait_timeout` method to deliver messages to each other. 
   * The `timeout` parameter determines how long `Clt` will wait in the event the socket is busy, while trying to send the message. To wait indefinitely you can use a `send_busywait` method instead, which will until success or error is encountered.
   * Notice that return type of this method is `SendStatus` which tell the user if the message was sent or timed out.
   * Another important details is that the method requires a `&mut` reference to the message being sent. This is intentional as the the more advanced features of the `ProtocolCore` trait allow you to modify the message before it is sent, for example update a `sequence number` or `timestamp` or `checksum` etc. This use case is not covered in basic section of this example for simplicity sake.

9. `sleep` - at the end of the main function is so that the program has the time to spawn the `receiver` thread that will issue callbacks and log them into the terminal as `Clt` & `Svc` exchange messages.


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
# impl Messenger for MessageProtocol {
#     type RecvT = ExchangeDataModel;
#     type SendT = ExchangeDataModel;
#     #[inline(always)]
#     fn deserialize(frame: &[u8]) -> Result<Self::RecvT, std::io::Error> {
#         let frame = std::str::from_utf8(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, # e))?;
#         let frame = serde_json::from_str(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, # e))?;
#         Ok(frame)
#     }
#     #[inline(always)]
#     fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), # std::io::Error> {
#         let msg = serde_json::to_string(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, # e))?;
#         let mut msg = msg.into_bytes();
#         msg.push(b'\n');
#         let mut buf = [0_u8; MAX_MSG_SIZE];
#         buf[..msg.len()].copy_from_slice(&msg);
#         Ok((buf, msg.len()))
#     }
# }
# impl ProtocolCore for MessageProtocol {}
# impl Protocol for MessageProtocol {}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).try_init().unwrap();

    // common
    let addr = "127.0.0.1:8080";
    let timeout = std::time::Duration::from_secs(1);
    const MAX_MSG_SIZE: usize = 128;

    // svc
    let max_connections = std::num::NonZeroUsize::new(1).unwrap();
    
    // clt
    let retry_after = timeout / 10;
    let callback = LoggerCallback::new_ref();
    let protocol = MessageProtocol;

    let mut svc = Svc::<_, _, MAX_MSG_SIZE>::bind(
            addr, 
            max_connections, 
            callback.clone(), 
            protocol.clone(), 
            Some("svc")
            ).unwrap().into_sender_with_spawned_recver();

    let mut clt = Clt::<_, _, MAX_MSG_SIZE>::connect(
            addr, 
            timeout, 
            retry_after, 
            callback.clone(), 
            protocol, 
            Some("clt")
            ).unwrap().into_sender_with_spawned_recver();
    
    let mut ping = ExchangeDataModel::Ping(Ping {});
    let mut pong = ExchangeDataModel::Pong(Pong {});
    
    clt.send_busywait_timeout(&mut ping, timeout)
        .unwrap()
        .unwrap_completed();
    
    svc.send_busywait_timeout(&mut pong, timeout)
        .unwrap()
        .unwrap_completed();

    std::thread::sleep(timeout);
}

```