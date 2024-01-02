# Data Model

Lets first define a simple data model that both `Clt` & `Svc` will be able to send & receive. For simplicity of this example it is assumed that both `Clt` & `Svc` can `send` & `recv` same exact message type, however, under real conditions `Svc` & `Clt` would likely share some common message structures while also having some that only `Clt` or `Svc` would be able to `send` & `recv`. 

When creating a data model for a more realistic scenario where only a `Clt` might be able to `send` `LoginRequest` and `Svc` would only be able to `send` `LoginResponse` follow the same steps but in step `#3` define two `enum` structures, one for `Clt` and one for `Svc`.


1. Define `Ping` message type

```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Ping;
# fn main() {
#     let ping = Ping;
#     println!("{:?}", ping);
# }
```

2. Define `Pong` message type
   
```rust
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pong;
# fn main() {
#     let pong = Pong;
#     println!("{:?}", pong);
# }
```

3. Define `enum` structure that will represent valid message types that `Clt` & `Svc` can exchange.

```rust
# #[derive(Debug, serde::Serialize, serde::Deserialize)]
# struct Ping;
# #[derive(Debug, serde::Serialize, serde::Deserialize)]
# struct Pong;
#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum ExchangeDataModel {
    Ping(Ping),
    Pong(Pong),
}
# fn main() {
#     let dm = ExchangeDataModel::Ping(Ping);
#     println!("{:?}", dm);
#     let dm = ExchangeDataModel::Pong(Pong);
#     println!("{:?}", dm);
# }
```