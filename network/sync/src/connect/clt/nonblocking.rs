// use std::error::Error;

// use links_network_core::prelude::{ConId, MessengerNew};

// use crate::connect::messenger::nonblocking::{MessageSender, MessageRecver};

// pub struct CltSender<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> {
//     con_id: ConId,
//     writer: MessageSender<M, MAX_MESSAGE_SIZE>,
//     phantom: std::marker::PhantomData<M>,
// }

// pub struct CltReciver<M: MessengerNew, const MAX_MESSAGE_SIZE: usize> {
//     con_id: ConId,
//     writer: MessageRecver<M, MAX_MESSAGE_SIZE>,
//     phantom: std::marker::PhantomData<M>,
// }

// pub struct Clt {}

// impl Clt {
//     pub fn connect(addr: &str, name: Option<&str>) -> Result<(), Box<dyn Error>> {
        

//     }
// }
