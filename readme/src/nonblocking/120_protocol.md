# Protocol

The next step after defining the data model is to implement a `Protocol` trait. The purpose of this trait is to define several key functions for the `links_connect_nonblocking` library to be able to send & receive messages over the network link.

The `trait Protocol` itself is a super trait which consists of several other traits, namely:

1. `trait Framer` - implementation of this trait is going to provide the logic for determining if the incoming network buffer contains sufficient number of bytes to create a complete message. It can be based on a number of different strategies such as:
   1. `Fixed size` - when all messages being exchanges are of exact same size.
   2. `Delimiter based` - when each message sent being delimited by a `special` character or a sequence of characters. This will allow for each message to have a different length and we will use this strategy in our example. It is not the most efficient strategy but it is the easiest to understand and learn from.
   3. `Header based` - when each message sent contains a header section that specifies the length of the message. This is the most efficient strategy but it is also the most complex to implement.

2. `trait Messenger` - this implementation specifies what message type `Clt` & `Svc` will be able to send & receive, as well as, provide the logic for serializing & deserializing messages into bytes.

3. `trait ProtocolCore` - all of the methods in this trait already come with default implementation, which will be optimized away by the compiler unless overridden. The purpose of this trait is to provide hooks into various connection events such as `on_connect`, `on_disconnect`, `on_sent`, `on_recv`, etc. Application developer can override these methods to provide custom logic for handling these events. For example, `on_connect` can be used to execute a handshake between `Clt` & `Svc` instances, `on_disconnect` can be used to send a closing message sequence, `on_sent` & `on_recv` can be used to track connection state or gather telemetry data.
4. `trait Protocol` - this is a super trait that combines all of the above traits into a single trait. While also providing additional hooks into a network connection. For example, it allows you to configure an automatic `Heartbeat` message to be sent at a regular interval or to provide a automated reply when a specific message sequence is detected.

> Note: The `trait Protocol` methods will only be called if `Clt` & `Svc` instances are created in a `reference` counted mode. 
