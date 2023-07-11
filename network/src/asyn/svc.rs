// use log::info;
// use tokio::{net, spawn};
// pub struct Svc {
//     addr: String,
// }

// impl Svc {
//     pub fn new(addr: String) -> Self {
//         Svc { addr }
//     }
//     pub async fn run(&self) {
//         let listener = net::TcpListener::bind(&self.addr).await.unwrap();
//         loop {
//             let (mut socket, addr) = listener.accept().await.unwrap();
//             info!("Accepted New Connection: {:?}, {:?}", socket, addr);
//             self.service_client_connection(socket).await;
//             // spawn(async move {
//             //     let (mut rd, mut wr) = socket.split();
//             //     io::copy(&mut rd, &mut wr).await.unwrap();
//             // });
//         }
//     }
//     async fn service_client_connection(&self, mut socket: net::TcpStream) {
//         spawn(async move {});
//     }
// }

// async fn say_world() {
//     println!("world");
// }

// // #[tokio::main]
// // async fn main() {
// //     // Calling `say_world()` does not execute the body of `say_world()`.
// //     let op = say_world();

// //     // This println! comes first
// //     println!("hello");

// //     // Calling `.await` on `op` starts executing `say_world`.
// //     op.await;
// // }

// // fn main() {
// //     let mut rt = tokio::runtime::Runtime::new().unwrap();
// //     rt.block_on(async {
// //         println!("hello");
// //     })
// // }

// // macro_rules! block_on{
// //     ($any:expr) => {
// //         tokio_test::block_on($any)
// //     };
// // }

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[tokio::test]
//     async fn test_svc() {
//         say_world().await;
//     }

//     pub fn reverse(input: &str) -> String {
//         input.chars().rev().collect()
//     }
//     #[test]
//     fn blah() {
//         assert_eq!(reverse("hello"), "olleh");
//     }
// }
