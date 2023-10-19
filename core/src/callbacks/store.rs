use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use crate::{asserted_short_name, core::macros::short_type_name, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub enum Message<T: Debug> {
    Recv(T),
    Sent(T),
    // Failed(T),
}
impl<T: Debug> Message<T> {
    // pub fn as_ref(&self) -> &T {
    //     match self {
    //         Self::Recv(t) => t,
    //         Self::Sent(t) => t,
    //     }
    // }
    pub fn into_t(self) -> T {
        match self {
            Self::Recv(t) => t,
            Self::Sent(t) => t,
        }
    }
}

pub trait Storage<T: Debug+Send+Sync>: Debug+Send+Sync {
    fn on_msg(&self, cond_id: ConId, msg: Message<T>);
}

#[derive(Debug)]
pub struct StoreCallback<M: Messenger, INTO: Debug+Send+Sync, S: Storage<INTO>>
where INTO: From<M::RecvT>+From<M::SendT>
{
    storage: Arc<S>,
    phantom: std::marker::PhantomData<(INTO, M)>,
}
impl<INTO, M: Messenger, S: Storage<INTO>> StoreCallback<M, INTO, S>
where INTO: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync
{
    pub fn new_ref(storage: Arc<S>) -> Arc<Self> {
        Arc::new(Self {
            storage,
            phantom: std::marker::PhantomData,
        })
    }
}
impl<M: Messenger, INTO, S: Storage<INTO>+'static> CallbackRecvSend<M> for StoreCallback<M, INTO, S> where INTO: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync+'static
{}
impl<M: Messenger, INTO, S: Storage<INTO>+'static> CallbackRecv<M> for StoreCallback<M, INTO, S>
where INTO: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync+'static
{
    #[inline(always)]
    fn on_recv(&self, con_id: &ConId, msg: &<M as Messenger>::RecvT) {
        let msg = msg.clone();
        self.storage
            .on_msg(con_id.clone(), Message::Recv(INTO::from(msg)));
    }
}
impl<M: Messenger, INTO, S: Storage<INTO>> CallbackSend<M> for StoreCallback<M, INTO, S>
where INTO: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync+'static
{
    #[inline(always)]
    fn on_sent(&self, con_id: &ConId, msg: &<M as Messenger>::SendT) {
        let msg = msg.clone();
        self.storage
            .on_msg(con_id.clone(), Message::Sent(INTO::from(msg)));
    }
}
impl<M: Messenger, INTO, S: Storage<INTO>> Display for StoreCallback<M, INTO, S>
where INTO: From<M::RecvT>+From<M::SendT>+Debug+Send+Sync
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}>",
            asserted_short_name!("StoreCallback", Self),
            short_type_name::<INTO>()
        )
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;
    use std::sync::Arc;
    use std::thread::Builder;

    use log::info;

    use crate::unittest::setup::{self, messenger::CltTestMessenger, model::*};

    use crate::prelude::*;

    #[test]
    fn test_callback() {
        setup::log::configure();
        #[derive(Debug)]
        struct LogStore<T>(std::marker::PhantomData<T>);
        impl<T> LogStore<T> {
            pub fn new_ref() -> Arc<Self> {
                Arc::new(Self(std::marker::PhantomData))
            }
        }
        impl<T: Debug+Send+Sync> Storage<T> for LogStore<T> {
            fn on_msg(&self, cond_id: ConId, msg: Message<T>) {
                info!("{}: {:?}", cond_id, msg);
            }
        }

        let log_store = LogStore::<TestMsg>::new_ref();

        Builder::new()
            .name("Callback-Thread".to_owned())
            .spawn({
                let log_store = log_store.clone();
                move || {
                    let clbk = StoreCallback::<CltTestMessenger, _, _>::new_ref(log_store);
                    let msg = TestCltMsg::Dbg(TestCltMsgDebug::new(b"hello".as_slice()));
                    clbk.on_sent(&ConId::default(), &msg);
                    let msg = TestSvcMsg::Dbg(TestSvcMsgDebug::new(b"hello".as_slice()));
                    clbk.on_recv(&ConId::default(), &msg);
                }
            })
            .unwrap();
    }
}
