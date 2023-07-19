use std::{error::Error, sync::Arc};

use framing::{Callback, ProtocolHandler};
use log::{error, info};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::asyn::clt::Clt;

use crate::asyn::con_msg::ConId;

// pub type SvcReaderRef<MESSENGER, FRAMER> = Arc<Mutex<Option<MessageRecver<MESSENGER, FRAMER>>>>;
// pub type SvcWriterRef<MESSENGER, const MAX_MSG_SIZE: usize> = Arc<Mutex<Option<MessageSender<MESSENGER, MAX_MSG_SIZE>>>>;

// pub type CallbackRef<HANDLER> = Arc<Mutex<impl Callback<HANDLER>>>;

#[derive(Debug)]
pub struct Svc<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> {
    // reader: SvcReaderRef<HANDLER>,
    // writer: SvcWriterRef<HANDLER>,
    phantom: std::marker::PhantomData<HANDLER>,
}

impl<HANDLER: ProtocolHandler, const MAX_MSG_SIZE: usize> Svc<HANDLER, MAX_MSG_SIZE> {
    pub async fn new(
        addr: &str,
        callback: impl Callback<HANDLER>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let con_id = ConId::Svc(addr.to_owned());
        let lis = TcpListener::bind(&addr).await?;
        info!("{:?} bound successfully", con_id);

        let callback = Arc::new(Mutex::new(callback));
        tokio::spawn(async move {
            info!("{:?} accept loop started", con_id);
            match Self::run(lis, callback).await {
                Ok(()) => info!("{:?} accept loop stopped", con_id),
                Err(err) => error!("{:?} accept loop exit err: {:?}", con_id, err),
            }
        });
        Ok(())
    }
    async fn run(
        lis: TcpListener,
        callback: Arc<Mutex<impl Callback<HANDLER>>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let con_id = ConId::Svc(format!(
                "{:?}<-{:?}",
                stream.local_addr()?,
                stream.peer_addr()?,
            ));

            let _clt =
                Clt::<HANDLER, MAX_MSG_SIZE>::from_stream(stream, callback.clone(), con_id.clone())
                    .await;
            // info!("{:?} STREAM STOPPED", con_id);
        }
    }
}

#[cfg(test)]
mod test {
    use framing::LoggerCallback;
    use soupbintcp4::prelude::*;

    use super::*;
    use crate::unittest::setup;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_svc() {
        setup::log::configure();
        let addr = &setup::net::default_addr();
        type SoupBinNative = SoupBinProtocolHandler<NoPayload>;
        const MAX_MSG_SIZE: usize = 1024;
        let logger = LoggerCallback::<SoupBinNative>::new();

        let svc = Svc::<SoupBinNative, MAX_MSG_SIZE>::new(addr, logger).await;
        info!("svc: {:?}", svc);
        // svc.send(SoupBinMsg::dbg(b"hello world from server!")).await;
        sleep(Duration::from_secs(100)).await;
    }
}
