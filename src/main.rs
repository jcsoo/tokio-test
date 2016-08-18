#![allow(dead_code)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;
mod dns_query;

use mio::channel::channel;
use udp::UdpSocket;
use dns_query::Message;

use futures::Future;

use tokio::io::Readiness;
use tokio::reactor::{self, Reactor, Task, Tick};
use tokio::util::future::{pair, Complete};
use tokio::util::channel::Receiver;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::io;

type CompleteMessage = Complete<Message, ()>;

struct Resolver {
    socket: Option<UdpSocket>,
    addr: SocketAddr,
    tx: mio::channel::Sender<(String, CompleteMessage)>,
    rx: Receiver<(String, CompleteMessage)>,
    requests: HashMap<u16, CompleteMessage>,
    next_id: u16,
}

struct ResolverHandle {
    tx: mio::channel::Sender<(String, CompleteMessage)>,
}

impl Resolver {
    fn next_id(&mut self) -> u16 {
        let v = self.next_id;
        self.next_id += 1;
        v
    }
}

impl ResolverHandle {
    fn lookup(&self, host: String) -> Box<Future<Item = Message, Error = ()>> {
        println!("look up {}", host);
        let (c, v) = pair::<Message, ()>();
        let _ = self.tx.send((host, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {
        let socket = match self.socket.take() {
            Some(s) => s,
            None => UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap(),
        };

        while socket.is_writable() && self.rx.is_readable() {
            if let Some((host, c)) = self.rx.recv().unwrap() {
                let id = self.next_id();
                let buf = dns_query::build_query(id, &host);
                println!("{}: {} to {}", id, host, self.addr);
                if let Ok(_) = socket.send_to(&buf, &self.addr) {
                    self.requests.insert(id, c);
                } else {
                    let _ = self.tx.send((host, c));
                    break;
                }
            } else {
                break;
            }
        }
        while socket.is_readable() {
            let mut buf = [0u8; 512];
            if let Ok(_) = socket.recv_from(&mut buf) {
                let msg = dns_query::parse_response(&mut buf);
                // println!("message: {:?}", msg);
                let id = msg.get_id();
                if let Some(c) = self.requests.remove(&id) {
                    c.complete(msg);
                }
            } else {
                break;
            }
        }
        self.socket = Some(socket);
        Ok(Tick::WouldBlock)
    }
}

fn setup(addr: SocketAddr) -> ResolverHandle {
    // must be run within a reactor
    let (tx, rx) = channel::<(String, CompleteMessage)>();
    let tx2 = tx.clone();
    // reactor::oneshot(move || {
    let rx = Receiver::watch(rx).unwrap();

    let r = Resolver {
        socket: None,
        addr: addr,
        tx: tx.clone(),
        rx: rx,
        requests: HashMap::new(),
        next_id: 1000,
    };
    reactor::schedule(r).unwrap();
    // }).unwrap();

    ResolverHandle { tx: tx2 }
}

fn print_response(m: Message) {
    println!("msg: {:?}", m.get_answers());
}

fn main() {
    let reactor = Reactor::default().unwrap();
    reactor.handle().oneshot(move || {
        let resolver = setup("8.8.8.8:53".parse().unwrap());
        let r1 = resolver.lookup("google.com".to_owned())
            .map(print_response);

        let r2 = resolver.lookup("amazon.com".to_owned())
            .map(print_response);

        let r3 = resolver.lookup("apple.com".to_owned())
            .map(print_response);

        r1.join(r2)
            .join(r3)
            .map(|_| {
                reactor::shutdown().unwrap();
            })
            .forget();
    });
    reactor.run().unwrap();
}
