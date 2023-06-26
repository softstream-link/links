use std::{
    io::Read,
    net::TcpStream,
    thread::{self, sleep, JoinHandle},
    time::{Duration, Instant},
};

use log::info;

use super::callback::ReadCallback;

#[cfg(test)]
mod test {
    use std::{
        borrow::BorrowMut,
        cell::{Cell, RefCell},
        io::{Read, Write},
        iter::Once,
        net::TcpStream,
        rc::Rc,
        sync::Arc,
        thread::spawn,
        time::Duration, collections::HashMap,
    };

    use log::info;

    use crate::{tcp::callback::LoggerCallback, unittest::setup};

    #[test]
    fn test_steram() {
        setup::log::configure();
        // let mut stream = TcpStream::connect("tcpbin.com:4242").unwrap();
        let mut stream = TcpStream::connect("127.0.0.1:5000").unwrap();
        stream.set_nodelay(true).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        info!("stream: {:?}", stream);
        let size = stream.write(b"hello there").unwrap();
        info!("size: {:?}", size);
        stream.flush().unwrap();
        info!("flushed");
        let mut buf = [0; 100];
        let size = stream.read(&mut buf).unwrap();
        info!("read size: {:?}, buf: {:?}", size, &buf[..size])
    }

    #[test]
    fn test_copy() {
        let x = [2; 23];
        let y = x;
        println!("x: {:?}, y: {:?}", x, y);

        #[derive(Debug, Copy, Clone)]
        struct MyFav {
            a: i32,
            b: i32,
        };
        let x = MyFav { a: 1, b: 2 };
        let y = x;
        println!("x: {:?}, y: {:?}", x, y);
        let cl = move || {
            println!("y: {:?}", y);
        };
        println!("y: {:?}", y);
        spawn(cl);
    }

    #[test]
    fn test_refcell() {
        // {
        //     let c = Cell::new(false);
        //     println!("c: {:?}", c);
        //     c.set(true);
        //     println!("c: {:?}", c);
        // }

        // {
        //     let greeting = RefCell::new("hello".to_string());
        //     println!("greeting: {:?}", greeting);
        //     // let x  = greeting.borrow();
        //     // println!("x: {:?}", x);
        //     // let y = greeting.borrow();
        //     // println!("y: {:?}", y);
        //     {
        //         let mut z = greeting.borrow_mut();
        //         *z = "hola".to_string();
        //         println!("z: {:?}", z);
        //     }
        //     let y = greeting.borrow();
        //     println!("y: {:?}", y);
        // }

        {
            // OnceCell::new(); unstable
        }
    }
    #[test]
    fn test_rc() {
        #[derive(Debug)]
        struct Node {
            parent: RefCell<Option<Rc<Node>>>,
            children: RefCell<Vec<Rc<Node>>>,
            data: String,
        }
        impl Node {
            fn new_root() -> Rc<Node> {
                Rc::new(Node {
                    parent: RefCell::new(None),
                    children: RefCell::new(vec![]),
                    data: "root".to_owned(),
                })
            }
            fn new_node(data: &str) -> Rc<Node> {
                Rc::new(Node {
                    parent: RefCell::new(None),
                    children: RefCell::new(vec![]),
                    data: data.to_owned(),
                })
            }
            fn append_child(parent: Rc<Self>, node: Rc<Node>) {
                {
                    let mut children = parent.children.borrow_mut();
                    children.push(Rc::clone(&node));
                }
                node.parent.replace(Some(parent));
            }
        }
        let root = Node::new_root();
        println!("root: {:#?}", root);
        let child1 = Node::new_node("child1");
        let child2 = Node::new_node("child2");
        Node::append_child(Rc::clone(&root), Rc::clone(&child1));
        Node::append_child(Rc::clone(&root), Rc::clone(&child2));
        let child3 = Rc::clone(&child2);
        println!("child1: {:#?}", Rc::strong_count(&child2));
        drop(child3);
        println!("child1: {:#?}", Rc::strong_count(&child2));
        drop(child1);
        drop(child2);
        println!("root: {:#?}", root);

        // #[derive(Debug)]
        // struct MyFav {
        //     a: i32,
        //     b: i32,
        // }
        // {
        //     let x = Rc::new((MyFav { a: 1, b: 2 }));
        //     let y = x.clone();
        //     let z = Rc::clone(&x);
        //     println!("x: {:?}", x);
        //     println!("y: {:?}", y);
        //     println!("z: {:?}", z);
        //     drop(x);
        //     println!("y: {:?}", y);
        // }

        // type MyFavRef = Arc<RefCell<MyFav>>;
        // // let mut i = MyFav { a: 1, b: 2 };
        // // let x = MyFavRef::new(RefCell::new(MyFav { a: 1, b: 2 }));
        // let mut x = Arc::new(RefCell::new(MyFav { a: 1, b: 2 }));
        // let mut y = x.clone();
        // println!("x: {:?}, y: {:?}", x, y);
        // x.as_ref().borrow_mut().a = 3;
        // println!("x: {:?}, y: {:?}", x, y);
    }

    #[test]
    fn test_bytes() {
        use bytes::*;
        let mut buf = BytesMut::with_capacity(100);
        for i in 0..101 {
            buf.put_u8(1);
        }
        // buf.put(b"hello".as_slice());
        // buf.put_u16(1);
        println!("buf: {:?}", buf);
        println!("buf capacity : {:?}", buf.capacity());
        println!("buf len : {:?}", buf.len());

        for i in 0..101 {
            let x = buf.get_u8();
        }
        println!("buf: {:?}", buf);
        println!("buf capacity : {:?}", buf.capacity());
        println!("buf len : {:?}", buf.len());

        for i in 0..101 {
            buf.put_u8(1);
        }
        println!("buf: {:?}", buf);
        println!("buf capacity : {:?}", buf.capacity());
        println!("buf len : {:?}", buf.len());

        let mut a = buf.split();
        println!("buf: {:?}", buf);
        println!("buf capacity : {:?}", buf.capacity());
        println!("buf len : {:?}", buf.len());

        // let x = buf.get_u8();

        // let mut a = buf.split();
        println!("a: {:?}", a);
        println!("a capacity: {:?}", a.capacity());
        println!("a len: {:?}", a.len());
        let x = a.get_u16();
        println!("x: {:?}", x);
        println!("a: {:?}", a);
        println!("a capacity: {:?}", a.capacity());
        println!("a len: {:?}", a.len());
        let x = &a[..2];
        println!("x: {:?}", x);
        println!("a: {:?}", a);
        println!("a capacity: {:?}", a.capacity());
        println!("a len: {:?}", a.len());
        // println!("buf: {:?}", buf);
        // println!("buf capacity : {:?}", buf.capacity());
        // a.put(b"world".as_slice());
        // println!("a: {:?}", a);
        // println!("a capacity: {:?}", a.capacity());
        // println!("buf: {:?}", buf);
        // println!("buf capacity : {:?}", buf.capacity());
        

    }
}
