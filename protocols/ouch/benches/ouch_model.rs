

use std::alloc::System;

use byteserde::prelude::*;
use byteserde_derive::{ByteDeserializeSlice, ByteSerializeStack, ByteSerializedLenOf};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use links_ouch_async::prelude::*;
use links_soupbintcp_async::prelude::*;

#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, Debug)]
#[byteserde(peek(0, 1))]
pub enum OuchCltPld1 {
    #[byteserde(eq(PacketTypeEnterOrder::as_slice()))]
    Enter(EnterOrder),
    #[byteserde(eq(PacketTypeReplaceOrder::as_slice()))]
    Replace(ReplaceOrder),
    #[byteserde(eq(PacketTypeCancelOrder::as_slice()))]
    Cancel(CancelOrder),
    #[byteserde(eq(PacketTypeModifyOrder::as_slice()))]
    Modify(ModifyOrder),
    #[byteserde(eq(PacketTypeAccountQueryRequest::as_slice()))]
    AccQry(AccountQueryRequest),
    #[byteserde(eq(PacketTypeAccountQueryRequest::as_slice()))]
    X(OuchSvcPld),
}
#[rustfmt::skip]
#[derive(ByteSerializeStack, ByteDeserializeSlice, ByteSerializedLenOf, PartialEq, Clone, std::fmt::Debug)]
#[byteserde(peek(2, 1))]
enum SBCltMsg1 {
    #[byteserde(eq(PacketTypeUnsequencedData::as_slice()))]
    U(UPayload::<OuchCltPld>),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    S(OuchCltPld),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    X(EnterOrder),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    Z(OuchCltPld1),
    #[byteserde(eq(PacketTypeSequencedData::as_slice()))]
    W(UPayload::<OuchCltPld1>),
    // #[byteserde(eq(PacketTypeCltHeartbeat::as_slice()))]
    // HBeat(CltHeartbeat),
    // #[byteserde(eq(PacketTypeDebug::as_slice()))]
    // Dbg(Debug),
    // #[byteserde(eq(PacketTypeLoginRequest::as_slice()))]
    // Login(LoginRequest),
    // #[byteserde(eq(PacketTypeLogoutRequest::as_slice()))]
    // Logout(LogoutRequest),
}
fn ouch_order_ser(c: &mut Criterion) {


    let order = EnterOrder::new(
        1.into(),
        100.into(),
        b"IBM".as_slice().into(),
        15.into(),
        TimeInForce::market_hours(),
        Display::visible(),
        Capacity::agency(),
        IntMktSweepEligibility::eligible(),
        CrossType::continuous_market(),
        b"MY ORDER ID #1".as_slice().into(),
        OptionalAppendage::default(),
    );
    // let msg = UPayload::new(order); // 9ns

    // let msg: OuchCltMsg = order.into();
    ///////
    // let msg = order; // 9ns
    let msg = OuchCltPld1::Enter(order); // 10ns
    let msg = OuchCltPld1::X(OuchSvcPld::SysEvt(SystemEvent::default())); // 2s
    let msg = SBCltMsg1::W(UPayload::new(OuchCltPld1::X(OuchSvcPld::SysEvt(SystemEvent::default())))); //7ns seemd due to enum

    // let msg = UPayload::new(OuchCltPld::Enter(order)); // 10ns
    // let msg = OuchCltMsg::U(UPayload::new(OuchCltPld::Enter(order))); // 28ns // TODO WHY??
    // let msg = OuchCltMsg::Dbg(Debug::new(b"HELLO WORLD".as_slice().into()));  //2ns
    // let msg = OuchCltMsg::Login(LoginRequest::default()); // 3ns
    // let msg = SBCltMsg::<Nil>::U(UPayload::default()); //75ps
    // let msg = SBCltMsg::<SamplePayload>::U(UPayload::default());  //71ps

    // let msg = SBCltMsg1::Dbg(Debug::new(b"blah".as_slice().into()));
    // let msg = SBCltMsg1::U(UPayload::new(OuchCltPld::Enter(order)));
    // let msg = SBCltMsg1::S(OuchCltPld::Enter(order)); // 28ns
    // let msg = SBCltMsg1::X(order); //27ns

    c.bench_function("ouch_order_ser", |b| {
        b.iter(|| {
            black_box({
                let _: ([u8; MAX_FRAME_SIZE_OUCH_CLT_MSG], usize) = to_bytes_stack(&msg).unwrap();
                // let _: ([u8; 51], usize) = to_bytes_stack(&msg).unwrap();
            })
        })
    });
}

// fn ouch_order_des(c: &mut Criterion) {
//     let order = EnterOrder::new(
//         1.into(),
//         100.into(),
//         b"IBM".as_slice().into(),
//         15.into(),
//         TimeInForce::market_hours(),
//         Display::visible(),
//         Capacity::agency(),
//         IntMktSweepEligibility::eligible(),
//         CrossType::continuous_market(),
//         b"MY ORDER ID #1".as_slice().into(),
//         OptionalAppendage::default(),
//     );
//     // let msg = UPayload::new(order); // 9ns

//     // let msg: OuchCltMsg = order.into();
//     ///////
//     // let msg = order; // 9ns
//     // let msg = OuchCltPld::Enter(order); // 10ns
//     let msg = UPayload::new(OuchCltPld::Enter(order)); // 10ns
//                                                        // let msg = OuchCltMsg::U(UPayload::new(OuchCltPld::Enter(order))); // 28ns // TODO WHY??
//     let (arr, len): ([u8; MAX_FRAME_SIZE_OUCH_CLT_MSG], usize) = to_bytes_stack(&msg).unwrap();

//     c.bench_function("ouch_order_des", |b| {
//         b.iter(|| {
//             black_box({
//                 let _ = from_slice::<UPayload<OuchCltPld>>(&arr[..len]).unwrap();
//                 // let msg: OuchCltMsg = from_slice(&arr[..len]).unwrap();
//             })
//         })
//     });
// }

// criterion_group!(benches, ouch_order_ser, ouch_order_des);
criterion_group!(benches, ouch_order_ser);
criterion_main!(benches);
