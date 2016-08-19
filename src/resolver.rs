use mio;
use mio::channel::channel;
use udp::UdpSocket;
use dns_query;
use dns_query::Message;
use transport::DnsTransport;

use futures::Future;

use tokio::Service;
use tokio::io::{Transport};
use tokio::reactor::{self, Task, Tick};
use tokio::util::future::{pair, Complete};
use tokio::util::channel::Receiver;
use tokio::proto::pipeline::Frame;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::io;

static DNS_REQ_ID: AtomicUsize = ATOMIC_USIZE_INIT;

type DnsRequest = (Message, CompleteMessage);
type CompleteMessage = Complete<Message, ()>;

pub fn next_request_id() -> u16 {
    DNS_REQ_ID.fetch_add(1, Ordering::Relaxed) as u16
}

struct Resolver {
    transport: DnsTransport,    
    tx: mio::channel::Sender<DnsRequest>,
    rx: Receiver<DnsRequest>,
    requests: HashMap<u16, CompleteMessage>,    
}

pub struct ResolverHandle {
    tx: mio::channel::Sender<DnsRequest>,
}

impl ResolverHandle {        
    pub fn lookup(&self, host: String) -> Box<Future<Item = Message, Error = ()>> {                
        self.call(dns_query::build_query_message(dns_query::any_query(&host)))
    }
}

impl Service for ResolverHandle {
    type Req = Message;
    type Resp = Message;
    type Error = ();
    type Fut = Box<Future<Item=Self::Resp, Error=Self::Error>>;

    fn call(&self, req: Message) -> Self::Fut {
        let (c, v) = pair::<Message, ()>();
        let _ = self.tx.send((req, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {
        self.transport.flush().unwrap();
        
        while let Some((mut msg, c)) = self.rx.recv().unwrap() {
            let id = msg.id(next_request_id()).get_id();
            try!(self.transport.write(Frame::Message(msg))); 
            self.requests.insert(id, c);            
        }

        if self.requests.len() > 0 {
            while let Ok(Some(Frame::Message(msg))) = self.transport.read() {
                if let Some(c) = self.requests.remove(&msg.get_id()) {
                    c.complete(msg);
                }
            }
        }
        
        self.transport.flush().unwrap();
        Ok(Tick::WouldBlock)
    }
}

pub fn resolver(addr: SocketAddr) -> ResolverHandle {
    // must be run within a reactor
    let (tx, rx) = channel::<DnsRequest>();
    let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
    let transport = DnsTransport::new(socket, addr);        
    {
        let r = Resolver {
            transport: transport,            
            tx: tx.clone(),
            rx: Receiver::watch(rx).unwrap(),
            requests: HashMap::new(),
        };
        reactor::schedule(r).unwrap();
    }
    
    ResolverHandle { tx: tx }
}
