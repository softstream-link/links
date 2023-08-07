use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
use std::fmt::Debug;
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    username: UserName,
    password: Password,
    session_id: SessionId,
    hbeat_timeout: Arc<Mutex<Option<TimeoutMs>>>,
    phantom: std::marker::PhantomData<PAYLOAD>,
}

impl<PAYLOAD> SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId) -> Arc<Self> {
            Arc::new(Self { username, password, session_id, hbeat_timeout: Arc::new(Mutex::new(None)), phantom: std::marker::PhantomData,})
        }
}

impl<PAYLOAD> Framer for SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[inline]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}

impl<PAYLOAD> Messenger for SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    type SendT = SBSvcMsg<PAYLOAD>;
    type RecvT = SBCltMsg<PAYLOAD>;
}

impl<PAYLOAD> Protocol for SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    async fn handshake<
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &self,
        clt: &Clt<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        let msg = clt.recv().await?;
        if let Some(SBCltMsg::Login(req)) = msg {
            // info!("{}<-{:?}", clt.con_id(), req);
            if (req.username != self.username) || (req.password != self.password) {
                clt.send(&mut SBSvcMsg::login_rej_not_auth()).await?;
                return Err(format!("{} Not Authorized", clt.con_id()).into());
            }
            if req.session_id != self.session_id {
                clt.send(&mut SBSvcMsg::login_rej_ses_not_avail()).await?;
                #[rustfmt::skip]  return Err(format!("{} '{}' No Session Avail", clt.con_id(),req.session_id).into());
            }
            let mut hbeat_timeout = self.hbeat_timeout.lock().await;
            *hbeat_timeout = Some(req.hbeat_timeout);
            // TODO what is correct sequence number to send ?
            clt.send(&mut SBSvcMsg::login_acc(self.session_id, 0.into()))
                .await?;
        } else {
            #[rustfmt::skip] return Err(format!("{} Invalid Handshake unexpected msg: {:?}", clt.con_id(), msg).into());
        }
        Ok(())
    }

    async fn keep_alive_loop<
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &self,
        clt: CltSender<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        let hbeat_timeout: u64 = { // drops the lock
            let hbeat_timeout = *self.hbeat_timeout.lock().await;
            hbeat_timeout.unwrap().into()  // TODO better then unwrap
        };
        let mut msg = SBSvcMsg::HBeat(SvcHeartbeat::default());
        loop {
            clt.send(&mut msg).await?;
            tokio::time::sleep(Duration::from_millis(hbeat_timeout)).await;
        }
    }
}
