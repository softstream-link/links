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
    use super::*;
    #[rustfmt::skip]
    #[derive(Debug, Clone)]
    pub struct SBSvcProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    { 
        phantom: std::marker::PhantomData<PAYLOAD> 
    }

    #[rustfmt::skip]
    impl<PAYLOAD> SBSvcProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {

    }

    #[rustfmt::skip]
    impl<PAYLOAD> Framer for SBSvcProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        #[inline]
        fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
            SoupBinFramer::get_frame(bytes)
        }
    }

    #[rustfmt::skip]
    impl<PAYLOAD> Messenger for SBSvcProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        type SendT = SBSvcMsg<PAYLOAD>;
        type RecvT = SBCltMsg<PAYLOAD>;
    }

    #[rustfmt::skip]
    impl<PAYLOAD> Protocol for SBSvcProtocol<PAYLOAD>
    where 
        PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
    {
        
    }
}
