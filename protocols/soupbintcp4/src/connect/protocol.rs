use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::{SBMsg, SoupBinFramer};


#[rustfmt::skip]
impl<PAYLOAD> Messenger for SoupBinProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    type SendMsg = SBMsg<PAYLOAD>;
    type RecvMsg = SBMsg<PAYLOAD>;
}

#[rustfmt::skip]
impl<PAYLOAD> Framer for SoupBinProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    #[inline]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}
#[rustfmt::skip]
impl<PAYLOAD> Protocol for SoupBinProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    
}


#[rustfmt::skip]
#[derive(Debug, Clone)]
pub struct SoupBinProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{ 
    phantom: std::marker::PhantomData<PAYLOAD> 
}

#[rustfmt::skip]
impl<PAYLOAD> SoupBinProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    pub fn new_svc() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}
