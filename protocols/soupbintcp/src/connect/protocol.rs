use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;
use std::fmt::Debug;
use std::{error::Error, sync::Arc};

use crate::prelude::*;


pub use svc::*;
pub mod svc {

    use super::*;

    #[derive(Debug, Clone)]
    pub struct SBSvcAdminProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        username: UserName,
        password: Password,
        session_id: SessionId,
        phantom: std::marker::PhantomData<PAYLOAD>,
    }

    impl<PAYLOAD> SBSvcAdminProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        #[rustfmt::skip]
        pub fn new_ref(username: UserName, password: Password, session_id: SessionId) -> Arc<Self> {
            Arc::new(Self { username, password, session_id, phantom: std::marker::PhantomData,})
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
                // TODO what is correct sequence number to send ?
                clt.send(&mut SBSvcMsg::login_acc(self.session_id, 0.into()))
                    .await?;
            } else {
                #[rustfmt::skip] return Err(format!("{} Invalid Handshake unexpected msg: {:?}", clt.con_id(), msg).into());
            }
            Ok(())
        }
    }
}
