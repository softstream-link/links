use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
use log::warn;
use tokio::sync::Mutex;
use tokio::task::yield_now;
use tokio::time::Instant;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;
use std::{error::Error, sync::Arc};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SBCltAdminProtocol<SendPayload,RecvPayload>
where 
    SendPayload: ByteDeserializeSlice<SendPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    username: UserName,
    password: Password,
    session_id: SessionId,
    sequence_number: SequenceNumber,
    hbeat_interval: Duration,
    hbeat_tolerance_factor: f64,
    recv_tracker: Arc<Mutex<Option<EventIntervalTracker>>>,
    phantom: PhantomData<(SendPayload, RecvPayload)>,
}

impl<SendPayload, RecvPayload> SBCltAdminProtocol<SendPayload, RecvPayload>
where 
    SendPayload: ByteDeserializeSlice<SendPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId, sequence_number: SequenceNumber, hbeat_interval: Duration, hbeat_tolerance_factor: f64) -> Arc<Self> {
        Arc::new(Self {username, password, session_id, sequence_number, hbeat_interval, hbeat_tolerance_factor,
            recv_tracker: Arc::new(Mutex::new(None)), 
            phantom: PhantomData,
        })
    }
}

impl<SendPayload, RecvPayload> Framer for SBCltAdminProtocol<SendPayload, RecvPayload>
where 
    SendPayload: ByteDeserializeSlice<SendPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    #[inline]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}

impl<SendPayload, RecvPayload> Messenger for SBCltAdminProtocol<SendPayload, RecvPayload>
where 
    SendPayload: ByteDeserializeSlice<SendPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
{
    type SendT = SBCltMsg<SendPayload>;
    type RecvT = SBSvcMsg<RecvPayload>;
}

impl<SendPayload, RecvPayload> Protocol for SBCltAdminProtocol<SendPayload, RecvPayload>
where 
    SendPayload: ByteDeserializeSlice<SendPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
    RecvPayload: ByteDeserializeSlice<RecvPayload>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static,
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
        clt.send(&mut SBCltMsg::login(self.username, self.password, self.session_id, self.sequence_number, (self.hbeat_interval.as_millis() as u16).into(),)).await?;
        let msg = clt.recv().await?;
        match msg {
            Some(SBSvcMsg::LoginAcc(_)) => {
                let recv_tracker = EventIntervalTracker::new(clt.con_id().clone(), self.hbeat_interval, self.hbeat_tolerance_factor);
                *self.recv_tracker.lock().await = Some(recv_tracker);
                Ok(())
            },
            Some(SBSvcMsg::LoginRej(msg)) => Err(format!("{} msg: {:?}", clt.con_id(), msg).into()),
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
            tokio::time::sleep(self.hbeat_interval).await;
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
