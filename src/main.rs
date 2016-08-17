#![allow(dead_code, unused_imports)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;
mod dns_query;

use udp::UdpSocket;
use dns_query::Message;

use tokio::Service;
use tokio::reactor::{self, Reactor, Task, Tick};
use tokio::tcp::TcpStream;

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
} 

impl Resolver {
    fn new(addr: SocketAddr) -> Resolver {        
        let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
        Resolver{ socket: socket, addr: addr, state: State::Sending }
    }
}

impl Task for Resolver {
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
        Ok(Tick::WouldBlock)        
    } 
}

fn main() {    
    let reactor = Reactor::default().unwrap();     
    reactor.handle().oneshot(|| {
        let resolver = Resolver::new("8.8.8.8:53".parse().unwrap());
        reactor::schedule(resolver).unwrap();
    });
    reactor.run().unwrap();
}
    