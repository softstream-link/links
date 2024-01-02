# Launching Svc

Now that we have the `Framer` & `Messenger` traits implemented, we can launch the `Svc` instance by binding it on a specific `port`. However, we still need to go over a few parameters:

1. `addr` - this is an ip/port that `Svc` will bind to.
2. `max_connections` - this is a maximum number of connections that `Svc` will accept. If an additional `Clt` will attempt to connect to `Svc` after `max_connections` has been reached, `Svc` will reject the connection until one of the existing connections is closed.
3. `callback` - this is a callback that will be used by `Svc` to notify application developer when `Svc` receives messages from `Clt`. You can define your own callbacks which capture the messages being `sent/received` or you can use a number of handy callbacks included with the library, please see documentation for more details. In the example below we will use library provided `LoggerCallback` that will simply log all messages `received` & `sent` by `Svc`.
4. `protocol` - this is an instance of the protocol that we created earlier.
5. `name` - this is an optional name that will be used by `LoggerCallback` to identify `Svc` instance in the logs.
6. `MAX_MSG_SIZE` - this determines the size of the maximum size byte array you can allocate in the `Messenger::serialize` method call


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
#         let frame = std::str::from_utf8(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
#         let frame = serde_json::from_str(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
#         Ok(frame)
#     }
#     #[inline(always)]
#     fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), std::io::Error> {
#         let msg = serde_json::to_string(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
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

    let addr = "127.0.0.1:8080";
    let max_connections = std::num::NonZeroUsize::new(1).unwrap();
    let callback = LoggerCallback::new_ref();
    let protocol = MessageProtocol;
    let name = Some("svc");
    const MAX_MSG_SIZE: usize = 128;

    let mut svc = Svc::<_, _, MAX_MSG_SIZE>::bind(
            addr, 
            max_connections, 
            callback,
            protocol, 
            name,
            ).unwrap();
}
```

Note that at this point `Svc` instance will internally maintain a `TcpListener` which accepts new `Clt` connections and a `Pool` of these connections. To use it in this form one need to use a combination of the following methods:

1. `svc.accept_into_pool_busywait_timeout()` - will return once a new 'Clt' connection is established or timeout is reached. The new connection will be added to the `Pool` of internal connections.
    > * if you wish to get the `Clt` instance instead of adding it to the `Pool` use `svc.accept_busywait_timeout()` instead.
    > * also if you try to use `svc.send_busywait_timeout()` or `svc.recv_busywait_timeout()` before `svc.accept_into_pool_busywait_timeout()` you will get an error indicating that the `Pool` is empty.

2. `svc.send_busywait_timeout()` - will round-robin `Clt`s in the internal `Pool` to delegate this call. If you set max_connections to `1` it will always send to using same `Clt` instance until `Clt` closes the connection.
3. `svc.recv_busywait_timeout()` - will round-robin `Clt`s in the internal `Pool` to delegate this call. If you set max_connections to `1` it will always send to using same `Clt` instance until `Clt` closes the connection.

If you wish to to delegate handing if accepting new connections and listening to the incoming messages you can use a convenience method `svc.into_sender_with_spawned_recver()` that will spawn a thread to do this and will return a `Sender` instance which will only have ability to call `send_busywait_timeout()` method

