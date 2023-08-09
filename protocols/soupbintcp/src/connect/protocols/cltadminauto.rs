use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
use log::warn;
use tokio::sync::Mutex;
use tokio::task::yield_now;
use tokio::time::Instant;
use std::fmt::Debug;
use std::time::Duration;
use std::{error::Error, sync::Arc};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    username: UserName,
    password: Password,
    session_id: SessionId,
    sequence_number: SequenceNumber,
    hbeat_timeout: Duration,
    last_recv_inst: Arc<Mutex<Instant>>,
    phantom: std::marker::PhantomData<PAYLOAD>,
}

impl<PAYLOAD> SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId, sequence_number: SequenceNumber, hbeat_timeout: Duration) -> Arc<Self> {
            Arc::new(Self {username, password, session_id, sequence_number, hbeat_timeout, 
                last_recv_inst: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60*60*24))), // 1 day ago
                phantom: std::marker::PhantomData,})
        }
}

impl<PAYLOAD> Framer for SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[inline]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}

impl<PAYLOAD> Messenger for SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    type SendT = SBCltMsg<PAYLOAD>;
    type RecvT = SBSvcMsg<PAYLOAD>;
}

impl<PAYLOAD> Protocol for SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    async fn handshake<
        's,
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &'s self,
        clt: &'s Clt<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        #[rustfmt::skip] 
        clt.send(&mut SBCltMsg::login(self.username, self.password, self.session_id, self.sequence_number, (self.hbeat_timeout.as_millis() as u16).into(),)).await?;
        let msg = clt.recv().await?;
        match msg {
            Some(SBSvcMsg::LoginAcc(_)) => Ok(()),
            Some(SBSvcMsg::LoginRej(msg)) => {
                Err(format!("{} msg: {:?}", clt.con_id(), msg).into())
            }
            _ => Err(format!("{} Unexpected msg: {:?}", clt.con_id(), msg).into()),
        }
    }

    async fn keep_alive_loop<
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &self,
        clt: CltSender<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        let mut msg = SBCltMsg::HBeat(CltHeartbeat::default());
        loop {
            clt.send(&mut msg).await?;
            tokio::time::sleep(self.hbeat_timeout).await;
        }
    }

    async fn is_connected(&self, timeout: Option<Duration>) -> bool {
        let (now, timeout )= (Instant::now(), match timeout{
            Some(timeout) => timeout,
            None => Duration::from_secs(0),
        });
        
        loop {
            let since_last_recv = { *self.last_recv_inst.lock().await }.elapsed();
            if since_last_recv < self.hbeat_timeout{
                return true;
            }
            if now.elapsed() > timeout{
                return false;
            }else{
                yield_now().await;
            }
        }
    }
    async fn on_recv<'s>(&'s self, _con_id: &'s ConId, _msg: &'s Self::RecvT)  {
        *self.last_recv_inst.lock().await = Instant::now();
    }

}
