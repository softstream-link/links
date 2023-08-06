pub mod setup {
    pub mod model {
        use crate::prelude::*;
        use byteserde::prelude::*;

        #[rustfmt::skip]
        pub fn svc_msgs_default<T>() -> Vec<SBSvcMsg<T>>
        where
            T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + Default + std::fmt::Debug,
        {
            vec![
                SBSvcMsg::HBeat(SvcHeartbeat::default()),
                SBSvcMsg::Dbg(Debug::default()),
                SBSvcMsg::LoginAcc(LoginAccepted::default()),
                SBSvcMsg::LoginRej(LoginRejected::not_authorized()),
                SBSvcMsg::End(EndOfSession::default()),
                SBSvcMsg::SData(SequencedData::new(T::default())),
                SBSvcMsg::UData(UnsequencedData::new(T::default())),
            ]
        }

        #[rustfmt::skip]
        pub fn clt_msgs_default<T>() -> Vec<SBCltMsg<T>>
        where
            T: ByteSerializeStack + ByteDeserializeSlice<T> + ByteSerializedLenOf + PartialEq + Clone + Default + std::fmt::Debug,
        {
            vec![
                SBCltMsg::HBeat(CltHeartbeat::default()),
                SBCltMsg::Dbg(Debug::default()),
                SBCltMsg::Login(LoginRequest::default()),
                SBCltMsg::Logout(LogoutRequest::default()),
                SBCltMsg::SData(SequencedData::new(T::default())),
                SBCltMsg::UData(UnsequencedData::new(T::default())),
            ]
        }
    }
}
