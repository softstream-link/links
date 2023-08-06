use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use byteserde::prelude::*;
use links_network_async::prelude::*;

use crate::prelude::{SBCltMsg, SoupBinFramer};

#[rustfmt::skip]
impl<PAYLOAD> Messenger for SBProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    type SendT = SBCltMsg<PAYLOAD>;
    type RecvT = SBCltMsg<PAYLOAD>;
}

#[rustfmt::skip]
impl<PAYLOAD> Framer for SBProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    #[inline]
    fn get_frame(bytes: &mut BytesMut) -> Option<Bytes> {
        SoupBinFramer::get_frame(bytes)
    }
}
#[rustfmt::skip]
impl<PAYLOAD> Protocol for SBProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{
    
}

#[rustfmt::skip]
#[derive(Debug, Clone)]
pub struct SBProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{ 
    phantom: std::marker::PhantomData<PAYLOAD> 
}

#[rustfmt::skip]
impl<PAYLOAD> SBProtocol<PAYLOAD>
where 
    PAYLOAD: ByteDeserializeSlice<PAYLOAD> + ByteSerializeStack + ByteSerializedLenOf + PartialEq + Debug + Clone + Send + Sync + 'static,
{

}
