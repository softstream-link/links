pub mod setup {
    pub mod protocol {
        use crate::{
            core::{protocol::ProtocolCore, RecvNonBlocking, RecvStatus, SendNonBlocking, SendStatus},
            prelude::{Framer, Messenger, Protocol},
        };
        use links_core::{
            core::conid::ConnectionId,
            unittest::setup::{
                framer::{CltTestMessenger, SvcTestMessenger},
                model::*,
            },
        };
        use log::info;
        use std::{
            io::{Error, ErrorKind},
            time::Duration,
        };

        #[derive(Debug, Clone, Default)]
        pub struct SvcTestProtocolSupervised;
        impl Framer for SvcTestProtocolSupervised {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                SvcTestMessenger::get_frame_length(bytes)
            }
        }
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
        impl ProtocolCore for SvcTestProtocolSupervised {}
        impl Protocol for SvcTestProtocolSupervised {}

        /// Provides an [ProtocolCore::on_connected] implementation
        #[derive(Debug, Clone, Default)]
        pub struct SvcTestProtocolAuthAndHBeat;
        impl Framer for SvcTestProtocolAuthAndHBeat {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                SvcTestMessenger::get_frame_length(bytes)
            }
        }
        impl Messenger for SvcTestProtocolAuthAndHBeat {
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
        impl ProtocolCore for SvcTestProtocolAuthAndHBeat {
            fn on_connected<C: SendNonBlocking<Self> + RecvNonBlocking<Self> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
                info!("on_connected: {}", con.con_id());
                let timeout = Duration::from_secs(1);
                match con.recv_busywait_timeout(timeout)? {
                    RecvStatus::Completed(Some(CltTestMsg::Login(_login))) => {
                        // info!("{} {:?}", clt.con_id(), login);
                        let mut msg: SvcTestMsg = SvcTestMsgLoginAcpt::default().into();
                        match con.send_busywait_timeout(&mut msg, timeout)? {
                            SendStatus::Completed => Ok(()),
                            SendStatus::WouldBlock => Err(Error::new(ErrorKind::TimedOut, format!("{} Timed out sending Login Accept", con.con_id())))?,
                        }
                    }
                    RecvStatus::Completed(msg) => Err(Error::new(ErrorKind::InvalidData, format!("{} Expected Login Request instead got msg: {:?}", con.con_id(), msg))),
                    RecvStatus::WouldBlock => Err(Error::new(ErrorKind::TimedOut, format!("{} Timed out waiting for Login Request", con.con_id())))?,
                }
            }
        }
        impl Protocol for SvcTestProtocolAuthAndHBeat {
            fn conf_heart_beat_interval(&self) -> Option<Duration> {
                Some(Duration::from_millis(100))
            }
            fn send_heart_beat<S: SendNonBlocking<Self> + ConnectionId>(&self, sender: &mut S) -> Result<SendStatus, Error> {
                let mut msg: SvcTestMsg = SvcTestMsg::HBeat(Default::default());
                sender.send(&mut msg)
            }

            fn send_reply<S: SendNonBlocking<Self> + ConnectionId>(&self, msg: &<Self as Messenger>::RecvT, sender: &mut S) -> Result<(), Error> {
                if let CltTestMsg::Ping(_ping) = msg {
                    let mut msg = SvcTestPong::default().into();
                    sender.send_busywait_timeout(&mut msg, Duration::from_millis(100))?;
                }
                Ok(())
            }
        }

        #[derive(Debug, Clone, Default)]
        pub struct CltTestProtocolSupervised;
        impl Framer for CltTestProtocolSupervised {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                CltTestMessenger::get_frame_length(bytes)
            }
        }
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
        impl ProtocolCore for CltTestProtocolSupervised {}
        impl Protocol for CltTestProtocolSupervised {}

        /// Provides an [ProtocolCore::on_connected] implementation]
        #[derive(Debug, Clone, Default)]
        pub struct CltTestProtocolAuthAndHbeat;
        impl Framer for CltTestProtocolAuthAndHbeat {
            fn get_frame_length(bytes: &mut bytes::BytesMut) -> Option<usize> {
                CltTestMessenger::get_frame_length(bytes)
            }
        }
        impl Messenger for CltTestProtocolAuthAndHbeat {
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
        impl ProtocolCore for CltTestProtocolAuthAndHbeat {
            fn on_connected<C: SendNonBlocking<Self> + RecvNonBlocking<Self> + ConnectionId>(&self, con: &mut C) -> Result<(), Error> {
                info!("on_connected: {}", con.con_id());
                let timeout = Duration::from_secs(1);
                let mut msg: CltTestMsg = CltTestMsgLoginReq::default().into();
                con.send_busywait_timeout(&mut msg, timeout)?.unwrap_completed(); //send login request
                let _msg = con.recv_busywait_timeout(timeout)?.unwrap_completed_some(); // wait for login accept
                Ok(())
            }
        }
        impl Protocol for CltTestProtocolAuthAndHbeat {
            fn conf_heart_beat_interval(&self) -> Option<Duration> {
                Some(Duration::from_millis(100))
            }
            fn send_heart_beat<S: SendNonBlocking<Self> + ConnectionId>(&self, sender: &mut S) -> Result<SendStatus, Error> {
                let mut msg: CltTestMsg = CltTestMsg::HBeat(Default::default());
                sender.send(&mut msg)
            }
        }
    }
}
