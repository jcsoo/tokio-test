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
use tokio::util::channel::{Receiver};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::io;

type CompleteMessage = Complete<Message,()>;

struct Resolver {
    socket: UdpSocket,
    addr: SocketAddr,
    tx: mio::channel::Sender<(String, CompleteMessage)>,
    rx: Receiver<(String, CompleteMessage)>,
    requests: HashMap<u16, CompleteMessage>,
    next_id: u16,
} 

impl Resolver {
    fn new(addr: SocketAddr) -> Resolver {        
        let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
        let (tx, rx) = channel::<(String, CompleteMessage)>();     
        let rx = Receiver::watch(rx).unwrap();   
        Resolver{ 
            socket: socket, 
            addr: addr, 
            tx: tx,
            rx: rx,
            requests: HashMap::new(),
            next_id: 1000, 
        }
    }

    fn next_id(&mut self) -> u16 {
        let v = self.next_id;
        self.next_id += 1;
        v
    }

    fn lookup(&mut self, host: String) -> Box<Future<Item=Message, Error=()>> {
        println!("look up {}", host);
        let (c, v) = pair::<Message, ()>();
        let _ = self.tx.send((host, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {
        while self.socket.is_writable() && self.rx.is_readable() {
            if let Some((host, c)) = self.rx.recv().unwrap() {
                let id = self.next_id();
                let buf = dns_query::build_query(id, &host);
                println!("{}: {} to {}", id, host, self.addr);
                if let Ok(_) = self.socket.send_to(&buf, &self.addr) {
                    self.requests.insert(id, c);
                } else {
                    let _ = self.tx.send((host, c));
                    break;
                }
            } else {
                break;
            }
        }
        while self.socket.is_readable() {
            let mut buf = [0u8;512];
            if let Ok(_) = self.socket.recv_from(&mut buf) {
                let msg = dns_query::parse_response(&mut buf);                
                //println!("message: {:?}", msg);
                let id = msg.get_id();
                if let Some(c) = self.requests.remove(&id) {
                    c.complete(msg);
                }
            } else {
                break;
            }
        }
        Ok(Tick::WouldBlock)
    }
}

fn print_response(m: Message) {
    println!("msg: {:?}", m.get_answers());
}

fn main() {    
    let reactor = Reactor::default().unwrap();     
    reactor.handle().oneshot(|| {
        let mut resolver = Resolver::new("8.8.8.8:53".parse().unwrap());

        resolver
            .lookup("google.com".to_owned())
            .map(print_response)
            .forget();

        resolver
            .lookup("amazon.com".to_owned())
            .map(print_response)
            .forget();

        reactor::schedule(resolver).unwrap();
    });
    reactor.run().unwrap();
}
    