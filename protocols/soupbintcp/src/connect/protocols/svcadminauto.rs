use bytes::BytesMut;
use byteserde::prelude::*;
use links_async::prelude::*;
use log::warn;
use tokio::task::yield_now;
use tokio::time::Instant;
use std::fmt::Debug;
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SBSvcAdminProtocol<SendPayLoad, RecvPayload>
where 
    SendPayLoad: ByteDeserializeSlice<SendPayLoad>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    username: UserName,
    password: Password,
    session_id: SessionId,
    hbeat_interval: Arc<Mutex<Option<Duration>>>,
    hbeat_tolerance_factor: f64,
    recv_tracker: Arc<Mutex<Option<EventIntervalTracker>>>,
    phantom: std::marker::PhantomData<(SendPayLoad, RecvPayload)>,
}

impl<SendPayLoad, RecvPayload> SBSvcAdminProtocol<SendPayLoad, RecvPayload>
where 
    SendPayLoad: ByteDeserializeSlice<SendPayLoad>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId, hbeat_tolerance_factor: f64) -> Arc<Self> {
        Arc::new(Self { username, password, session_id, 
            hbeat_interval: Arc::new(Mutex::new(None)),
            hbeat_tolerance_factor, 
            recv_tracker: Arc::new(Mutex::new(None)), 
            phantom: std::marker::PhantomData,})
    }
}

impl<SendPayLoad, RecvPayload> Framer for SBSvcAdminProtocol<SendPayLoad, RecvPayload>
where 
    SendPayLoad: ByteDeserializeSlice<SendPayLoad>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    #[inline]
    fn get_frame_length(bytes: &mut BytesMut) -> Option<usize> {
        SoupBinFramer::get_frame_length(bytes)
    }
}

impl<SendPayLoad, RecvPayload> MessengerOld for SBSvcAdminProtocol<SendPayLoad, RecvPayload>
where 
    SendPayLoad: ByteDeserializeSlice<SendPayLoad>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    type SendT = SBSvcMsg<SendPayLoad>;
    type RecvT = SBCltMsg<RecvPayload>;
}

impl<SendPayLoad, RecvPayload> Protocol for SBSvcAdminProtocol<SendPayLoad, RecvPayload>
where 
    SendPayLoad: ByteDeserializeSlice<SendPayLoad>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    async fn handshake<
        's,
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecvOld<P>,
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
                #[rustfmt::skip]
                return Err(format!("{} '{}' No Session Avail", clt.con_id(),req.session_id).into());
            }
            // save hbeat from clt login req
            { // drop lock
                let hbeat_timeout = Duration::from_millis(req.hbeat_timeout_ms.into());
                *self.hbeat_interval.lock().await = Some(hbeat_timeout);
                *self.recv_tracker.lock().await = Some(EventIntervalTracker::new(clt.con_id().clone(), hbeat_timeout, self.hbeat_tolerance_factor));
            }
            // TODO what is correct sequence number to send ?
            clt.send(&mut SBSvcMsg::login_acc(self.session_id, 0.into()))
                .await?;
        } else {
            #[rustfmt::skip]
            return Err(format!("{} Invalid Handshake unexpected msg: {:?}", clt.con_id(), msg).into());
        }
        Ok(())
    }

    async fn keep_alive_loop<
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecvOld<P>,
        const MMS: usize,
    >(
        &self,
        clt: CltSenderAsync<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        let hbeat_timeout = { // drops the lock
            let hbeat_timeout = *self.hbeat_interval.lock().await;
            hbeat_timeout.unwrap_or_else(||panic!("self.hbeat_timeout is None, must be set to Some duration handshake"))
        };
        let mut msg = SBSvcMsg::HBeat(SvcHeartbeat::default());
        loop {
            clt.send(&mut msg).await?;
            tokio::time::sleep(hbeat_timeout).await;
        }
    }

    async fn is_connected(&self, timeout: Option<Duration>) -> bool {
        let (now, timeout )= (Instant::now(), match timeout{
            Some(timeout) => timeout,
            None => Duration::from_secs(0),
        });
        
        loop {
            let recv_tracker = (*self.recv_tracker.lock().await).clone();
            let is_heart_beating = {
                match recv_tracker{
                    Some(ref recv_tracker) => recv_tracker.is_within_tolerance_factor(),
                    None => panic!("self.recv_tracker is None, must be set to Some EventIntervalTracker during handshake"),
                }   
            };
            if is_heart_beating {
                return true;
            }else if now.elapsed() > timeout{
                let is_connected = match recv_tracker {
                    Some(ref recv_tracker) => format!("{}", recv_tracker),
                    None => "None".to_owned(),
                };
                warn!("{} timeout: {:?}", is_connected, timeout);
                return false;
            }else{
                yield_now().await;
            }
            
        }
    }
    #[inline(always)]
    async fn on_recv<'s>(&'s self, _con_id: &'s ConId, _msg: &'s Self::RecvT)  {
        match *self.recv_tracker.lock().await{
            Some(ref mut recv_tracker) => recv_tracker.occurred(),
            None => panic!("self.recv_tracker is None, must be set to Some EventIntervalTracker during handshake"),
        }        
    }
}
