use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
use tokio::task::yield_now;
use tokio::time::Instant;
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
    hbeat_timeout: Arc<Mutex<Option<Duration>>>,
    last_recv_inst: Arc<Mutex<Instant>>,
    phantom: std::marker::PhantomData<PAYLOAD>,
}

impl<PAYLOAD> SBSvcAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId) -> Arc<Self> {
            Arc::new(Self { username, password, session_id, hbeat_timeout: Arc::new(Mutex::new(None)), 
                last_recv_inst: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60*60*24))), // 1 day ago
                phantom: std::marker::PhantomData,})
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
        's,
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &'s self,
        clt: &'s Clt<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        let msg = clt.recv().await?;
        if let Some(SBCltMsg::Login(req)) = msg {
            // validate usr/pwd
            if (req.username != self.username) || (req.password != self.password) {
                clt.send(&mut SBSvcMsg::login_rej_not_auth()).await?;
                return Err(format!("{} Not Authorized", clt.con_id()).into());
            }
            // validate session
            if req.session_id != self.session_id {
                clt.send(&mut SBSvcMsg::login_rej_ses_not_avail()).await?;
                #[rustfmt::skip] return Err(format!("{} '{}' No Session Avail", clt.con_id(),req.session_id).into());
            }
            // save hbeat
            { // drop lock
                *self.hbeat_timeout.lock().await = Some(Duration::from_millis(req.hbeat_timeout_ms.into()));
            }
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
        let hbeat_timeout = { // drops the lock
            let hbeat_timeout = *self.hbeat_timeout.lock().await;
            hbeat_timeout.unwrap_or_else(||panic!("self.hbeat_timeout is None, must be set to Some duration handshake"))
        };
        let mut msg = SBSvcMsg::HBeat(SvcHeartbeat::default());
        loop {
            clt.send(&mut msg).await?;
            tokio::time::sleep(hbeat_timeout).await;
        }
    }

    async fn is_connected(&self, timeout: Option<Duration>) -> bool {
        // get hbeat_time out from established connection
        let hbeat_timeout = {
            match *self.hbeat_timeout.lock().await{
                Some(hbeat_timeout) => hbeat_timeout,
                None => return false, // None default and only Some after connection is established
            }
        };

        let (now, timeout )= (Instant::now(), match timeout{
            Some(timeout) => timeout,
            None => Duration::from_secs(0),
        });
        
        loop {
            let since_last_recv = { *self.last_recv_inst.lock().await }.elapsed();
            if since_last_recv < hbeat_timeout{
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
