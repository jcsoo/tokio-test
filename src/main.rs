#![allow(dead_code, unused_imports)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;
mod dns_query;

use udp::UdpSocket;
use dns_query::Message;

use futures::Future;

use tokio::Service;
use tokio::io::Readiness;
use tokio::reactor::{self, Reactor, Task, Tick};
use tokio::tcp::TcpStream;
use tokio::util::future::{pair, Complete, Val};

use std::collections::{VecDeque, HashMap};
use std::net::SocketAddr;
use std::io;

enum State {
    Sending,
    Receiving,
}

struct Resolver {
    socket: UdpSocket,
    addr: SocketAddr,
    state: State,
    hosts: VecDeque<(String, Complete<Message,()>)>,
    requests: HashMap<u16, Complete<Message,()>>,
    next_id: u16,
} 

impl Resolver {
    fn new(addr: SocketAddr) -> Resolver {        
        let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();        
        Resolver{ 
            socket: socket, 
            addr: addr, 
            state: State::Sending,
            hosts: VecDeque::new(),
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
        self.hosts.push_back((host, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {
        println!("tick:start");
        // if items and socket writeable
        while self.socket.is_writable() {
            if let Some((host, c)) = self.hosts.pop_front() {
                let id = self.next_id();
                let buf = dns_query::build_query(id, &host);
                println!("{}: {} to {}", id, host, self.addr);
                if let Ok(n) = self.socket.send_to(&buf, &self.addr) {
                    println!(" {} bytes sent", n);
                    self.requests.insert(id, c);
                } else {
                    self.hosts.push_front((host, c));
                    break;
                }
            } else {
                break;
            }
        }
        println!("readable? {}", self.socket.is_readable());
        while self.socket.is_readable() {
            println!("read...");
            let mut buf = [0u8;512];
            if let Ok(_) = self.socket.recv_from(&mut buf) {
                let msg = dns_query::parse_response(&mut buf);                
                println!("message: {:?}", msg);
                let id = msg.get_id();
                if let Some(c) = self.requests.remove(&id) {
                    println!("completing...");
                    c.complete(msg);
                    println!("done completing");
                }
            } else {
                break;
            }
        }
        println!("tick:end");
        Ok(Tick::WouldBlock)
    }

/*
   fn tick(&mut self) -> io::Result<Tick> {
        loop {
            match self.state {
                State::Sending => {
                    let buf = dns_query::build_query(1234, "google.com");
                    if let Ok(_) = self.socket.send_to(&buf, &self.addr) {
                        self.state = State::Receiving;
                    } else {
                        return Ok(Tick::WouldBlock)
                    }
                },
                State::Receiving => {
                    let mut buf = [0u8;512];
                    if let Ok(_) = self.socket.recv_from(&mut buf) {
                        let msg = dns_query::parse_response(&mut buf);
                        println!("message: {:?}", msg);
                        return Ok(Tick::Final);                    
                    } else {
                        return Ok(Tick::WouldBlock);
                    }
                }
            }
        }
    }
    */ 
}

fn main() {    
    let reactor = Reactor::default().unwrap();     
    reactor.handle().oneshot(|| {
        let mut resolver = Resolver::new("8.8.8.8:53".parse().unwrap());

        resolver.lookup("google.com".to_owned());

        reactor::schedule(resolver).unwrap();
    });
    reactor.run().unwrap();
}
    