#![feature(async_fn_in_trait)] // TODO remove once async stabilized
#![feature(return_position_impl_trait_in_trait)]

pub mod connect;
pub mod callbacks;
pub mod core;
pub mod prelude;

#[cfg(test)]
pub mod unittest;
