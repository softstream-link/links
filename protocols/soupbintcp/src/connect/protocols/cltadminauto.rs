use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
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
    hbeat_timeout: TimeoutMs,
    phantom: std::marker::PhantomData<PAYLOAD>,
}

impl<PAYLOAD> SBCltAdminProtocol<PAYLOAD>
where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
{
    #[rustfmt::skip]
    pub fn new_ref(username: UserName, password: Password, session_id: SessionId, sequence_number: SequenceNumber, hbeat_timeout: TimeoutMs) -> Arc<Self> {
            Arc::new(Self {username, password, session_id, sequence_number, hbeat_timeout, phantom: std::marker::PhantomData,})
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
        P: Protocol<SendT=Self::SendT, RecvT=Self::RecvT>,
        C: CallbackSendRecv<P>,
        const MMS: usize,
    >(
        &self,
        clt: &Clt<P, C, MMS>,
    ) -> Result<(), Box<dyn Error+Send+Sync>> {
        #[rustfmt::skip]
            clt.send(&mut SBCltMsg::login(self.username, self.password, self.session_id, self.sequence_number,self.hbeat_timeout,)).await?;
        let msg = clt.recv().await?;
        match msg {
            Some(SBSvcMsg::LoginAcc(_)) => return Ok(()),
            Some(SBSvcMsg::LoginRej(msg)) => {
                return Err(format!("{} msg: {:?}", clt.con_id(), msg).into())
            }
            _ => return Err(format!("{} Unexpected msg: {:?}", clt.con_id(), msg).into()),
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
        let hbeat_timeout: u64 = self.hbeat_timeout.into();
        let mut msg = SBCltMsg::HBeat(CltHeartbeat::default());
        loop {
            clt.send(&mut msg).await?;
            tokio::time::sleep(Duration::from_millis(hbeat_timeout)).await;
        }
    }
}
