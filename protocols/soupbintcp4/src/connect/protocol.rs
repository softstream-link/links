use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::*;

pub use clt::*;
pub mod clt {
    use super::*;
    #[rustfmt::skip]
    #[derive(Debug, Clone)]
    pub struct SBCltProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    { 
        phantom: std::marker::PhantomData<PAYLOAD> 
    }

    #[rustfmt::skip]
    impl<PAYLOAD> SBCltProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {

    }

    #[rustfmt::skip]
    impl<PAYLOAD> Framer for SBCltProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        #[inline]
        fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
            SoupBinFramer::get_frame(bytes)
        }
    }

    #[rustfmt::skip]
    impl<PAYLOAD> Messenger for SBCltProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        type SendT = SBCltMsg<PAYLOAD>;
        type RecvT = SBSvcMsg<PAYLOAD>;
    }

    #[rustfmt::skip]
    impl<PAYLOAD> Protocol for SBCltProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        
    }
}

pub use svc::*;
pub mod svc {
    use std::{any::type_name, error::Error, sync::Arc};

    use log::info;

    use super::*;

    #[derive(Debug, Clone)]
    pub struct SBSvcAdminAutoProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        username: UserName,
        password: Password,
        session_id: SessionId,
        sequence_number: Option<SequenceNumber>,
        hbeat_timeout: Option<TimeoutMs>,
        phantom: std::marker::PhantomData<PAYLOAD>,
    }

    impl<PAYLOAD> SBSvcAdminAutoProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        pub fn new_ref(username: UserName, password: Password, session_id: SessionId) -> Arc<Self> {
            Arc::new(Self {
                username,
                password,
                session_id,
                sequence_number: None,
                hbeat_timeout: None,
                phantom: std::marker::PhantomData,
            })
        }
    }

    impl<PAYLOAD> Framer for SBSvcAdminAutoProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        #[inline]
        fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
            SoupBinFramer::get_frame(bytes)
        }
    }

    impl<PAYLOAD> Messenger for SBSvcAdminAutoProtocol<PAYLOAD>
    where PAYLOAD: ByteDeserializeSlice<PAYLOAD>+ByteSerializeStack+ByteSerializedLenOf+PartialEq+Debug+Clone+Send+Sync+'static
    {
        type SendT = SBSvcMsg<PAYLOAD>;
        type RecvT = SBCltMsg<PAYLOAD>;
    }

    impl<PAYLOAD> Protocol for SBSvcAdminAutoProtocol<PAYLOAD>
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
            if let Some(SBCltMsg::Login(login_req)) = msg {
                info!("{}<-{:?}", clt.con_id(), login_req);
                if (login_req.username != self.username) || (login_req.password != self.password) {
                    clt.send(&mut SBSvcMsg::login_rej_not_auth()).await?;
                    return Err(format!("{} Not Authorized", clt.con_id()).into());
                }
                if login_req.session_id != self.session_id {
                    clt.send(&mut SBSvcMsg::login_rej_ses_not_avail()).await?;
                    return Err(format!("{} No Session Avail", clt.con_id()).into());
                }
                clt.send(&mut SBSvcMsg::login_acc(self.session_id, 1.into()))
                    .await?;
            } else {
                #[rustfmt::skip] return Err(format!("{} Invalid Handshake unexpected msg: {:?}", clt.con_id(), msg).into());
            }
            Ok(())
        }
    }
}
