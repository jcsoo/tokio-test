#![allow(dead_code, unused_imports)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;
mod dns_query;

use udp::UdpSocket;
use dns_query::Message;

use futures::*;
use tokio::Service;
use tokio::reactor::Reactor;

use std::net::SocketAddr;
use std::io;


struct Resolver {    
    addr: SocketAddr,
} 

impl Resolver {
    fn new(addr: SocketAddr) -> Resolver {
        //let socket = UdpSocket::bind("0.0.0.0:0".parse().unwrap()).unwrap();
        Resolver{addr: addr }
    }
}

impl Service for Resolver {
    type Req = String;
    type Resp = Message;
    type Error = io::Error;
    type Fut = Box<Future<Item=Self::Resp, Error=Self::Error>>;

    fn call(&self, _req: Self::Req) -> Self::Fut {
        finished(Message::new()).boxed()
    }
}

fn main() {
    //let reactor = Reactor::default();
    let resolver = Resolver::new("8.8.8.8:53".parse().unwrap());
    
    resolver
        .call("google.com".to_owned())
        .then(move |msg| {
            println!("response: {:?}", msg);
            Ok::<(),()>(())
        }).forget();
    
}