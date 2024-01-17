# Nonblocking

At a high level `links_nonblocking` implementation requires the following steps:

1. Define & implement Data Model
   
 2. Implement `Protocol`
   
    1. Implement `Framer` trait
    2. Implement `Messenger` trait
    3. Implement `ProtocolCore` trait
    4. Implement `Protocol` trait