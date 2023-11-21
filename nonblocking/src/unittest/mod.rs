pub mod setup {
    pub mod protocol {
        use crate::{
            core::{RecvNonBlocking, RecvStatus, SendNonBlocking, SendStatus},
            prelude::{Framer, Messenger, Protocol},
        };
        use std::{
            io::{Error, ErrorKind},
            sync::Arc,
            time::Duration,
        };
        // use crate::prelude::{CallbackRecvSend, Clt, Protocol};
        use crate::prelude::{CallbackRecvSend, Clt};
        use links_core::unittest::setup::{
            framer::{CltTestMessenger, SvcTestMessenger},
            model::{CltTestMsg, CltTestMsgLoginReq, SvcTestMsg, SvcTestMsgLoginAcpt},
        };
        use log::info;

        #[derive(Debug)]
        pub struct SvcTestProtocolAuth;
        impl SvcTestProtocolAuth {
            pub fn new_ref() -> Arc<Self> {
                Arc::new(Self {})
            }
        }
        impl Protocol for SvcTestProtocolAuth {
            fn on_connected<M: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
                let timeout = Duration::from_secs(1);
                match clt.recv_busywait_timeout(timeout)? {
                    RecvStatus::Completed(Some(CltTestMsg::Login(login))) => {
                        info!("{} {:?}", clt.con_id(), login);
                        let mut msg: SvcTestMsg = SvcTestMsgLoginAcpt::default().into();
                        match clt.send_busywait_timeout(&mut msg, timeout)? {
                            SendStatus::Completed => Ok(()),
                            SendStatus::WouldBlock => Err(Error::new(ErrorKind::TimedOut, format!("{} Timed out sending Login Accept", clt.con_id())))?,
                        }
                    }
                    RecvStatus::Completed(msg) => Err(Error::new(ErrorKind::InvalidData, format!("{} Expected Login Request instead got msg: {:?}", clt.con_id(), msg))),
                    RecvStatus::WouldBlock => Err(Error::new(ErrorKind::TimedOut, format!("{} Timed out waiting for Login Request", clt.con_id())))?,
                }
            }
        }
        impl Messenger for SvcTestProtocolAuth {
            type RecvT = <SvcTestMessenger as Messenger>::RecvT;
            type SendT = <SvcTestMessenger as Messenger>::SendT;
            #[inline]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, std::io::Error> {
                SvcTestMessenger::deserialize(frame)
            }
            #[inline]
            fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), std::io::Error> {
                SvcTestMessenger::serialize(msg)
            }
        }
        impl Framer for SvcTestProtocolAuth {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                SvcTestMessenger::get_frame_length(bytes)
            }
        }

        #[derive(Debug)]
        pub struct CltTestProtocolAuth;
        impl CltTestProtocolAuth {
            pub fn new_ref() -> Arc<Self> {
                Arc::new(Self {})
            }
        }
        impl Protocol for CltTestProtocolAuth {
            fn on_connected<M: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
                let mut msg: CltTestMsg = CltTestMsgLoginReq::default().into();
                clt.send_busywait(&mut msg)?;
                Ok(())
            }
        }
        impl Messenger for CltTestProtocolAuth {
            type RecvT = <CltTestMessenger as Messenger>::RecvT;
            type SendT = <CltTestMessenger as Messenger>::SendT;
            #[inline]
            fn deserialize(frame: &[u8]) -> Result<Self::RecvT, Error> {
                CltTestMessenger::deserialize(frame)
            }
            #[inline]
            fn serialize<const MAX_MSG_SIZE: usize>(msg: &Self::SendT) -> Result<([u8; MAX_MSG_SIZE], usize), Error> {
                CltTestMessenger::serialize(msg)
            }
        }
        impl Framer for CltTestProtocolAuth {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                CltTestMessenger::get_frame_length(bytes)
            }
        }
    }
}
