use links_nonblocking::prelude::*;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Ping;
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pong;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum ExchangeDataModel {
    Ping(Ping),
    Pong(Pong),
}

#[derive(Debug, Clone)] // Note Clone required for Protocol
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
impl Messenger for MessageProtocol {
    type RecvT = ExchangeDataModel;
    type SendT = ExchangeDataModel;
    #[inline(always)]
    fn deserialize(frame: &[u8]) -> Result<Self::RecvT, std::io::Error> {
        let frame = std::str::from_utf8(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let received = serde_json::from_str(frame).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(received)
    }
    #[inline(always)]
    fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), std::io::Error> {
        let msg = serde_json::to_string(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut msg = msg.into_bytes();
        msg.push(b'\n');
        let mut buf = [0_u8; MAX_MSG_SIZE];
        buf[..msg.len()].copy_from_slice(&msg);
        Ok((buf, msg.len()))
    }
}
impl ProtocolCore for MessageProtocol {}
impl Protocol for MessageProtocol {}

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

    #[rustfmt::skip]
    let mut svc = Svc::<_, _, MAX_MSG_SIZE>::bind(
        addr, 
        max_connections, 
        callback.clone(), 
        protocol.clone(), 
        Some("svc")
    )
    .unwrap()
    .into_sender_with_spawned_recver();

    #[rustfmt::skip]
    let mut clt = Clt::<_, _, MAX_MSG_SIZE>::connect(
        addr, 
        timeout, 
        retry_after,
        callback.clone(), 
        protocol, 
        Some("clt")
    )
    .unwrap()
    .into_sender_with_spawned_recver();
    
    let mut ping = ExchangeDataModel::Ping(Ping {});
    clt.send_busywait_timeout(&mut ping, timeout).unwrap().unwrap_completed();
    let mut pong = ExchangeDataModel::Pong(Pong {});
    svc.send_busywait_timeout(&mut pong, timeout).unwrap().unwrap_completed();

    std::thread::sleep(timeout);
}
