pub mod setup {
    pub mod protocol {
        use crate::{
            core::{RecvNonBlocking, RecvStatus, SendNonBlocking, SendStatus},
            prelude::{Framer, Messenger, Protocol},
        };
        use std::{
            io::{Error, ErrorKind},
            time::Duration,
        };
        // use crate::prelude::{CallbackRecvSend, Clt, Protocol};
        use crate::prelude::{CallbackRecvSend, Clt};
        use links_core::unittest::setup::{
            framer::{CltTestMessenger, SvcTestMessenger},
            model::{CltTestMsg, CltTestMsgLoginReq, SvcTestMsg, SvcTestMsgLoginAcpt},
        };

        #[derive(Debug, Clone, Default)]
        pub struct SvcTestProtocolSupervised;
        impl Protocol for SvcTestProtocolSupervised {}
        impl Messenger for SvcTestProtocolSupervised {
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
        impl Framer for SvcTestProtocolSupervised {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                SvcTestMessenger::get_frame_length(bytes)
            }
        }

        #[derive(Debug, Clone, Default)]
        pub struct SvcTestProtocolAuth;
        impl Protocol for SvcTestProtocolAuth {
            fn on_connected<M: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
                let timeout = Duration::from_secs(1);
                match clt.recv_busywait_timeout(timeout)? {
                    RecvStatus::Completed(Some(CltTestMsg::Login(_login))) => {
                        // info!("{} {:?}", clt.con_id(), login);
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

        #[derive(Debug, Clone, Default)]
        pub struct CltTestProtocolSupervised;
        impl Protocol for CltTestProtocolSupervised {}
        impl Messenger for CltTestProtocolSupervised {
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
        impl Framer for CltTestProtocolSupervised {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                CltTestMessenger::get_frame_length(bytes)
            }
        }
        #[derive(Debug, Clone, Default)]
        pub struct CltTestProtocolAuth;
        impl Protocol for CltTestProtocolAuth {
            fn on_connected<M: Protocol<SendT = Self::SendT, RecvT = Self::RecvT>, C: CallbackRecvSend<M>, const MAX_MSG_SIZE: usize>(&self, clt: &mut Clt<M, C, MAX_MSG_SIZE>) -> Result<(), Error> {
                let timeout = Duration::from_secs(1);
                let mut msg: CltTestMsg = CltTestMsgLoginReq::default().into();
                clt.send_busywait_timeout(&mut msg, timeout)?.unwrap_completed(); //send login request
                let _msg = clt.recv_busywait_timeout(timeout)?.unwrap_completed_some(); // wait for login accept
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
