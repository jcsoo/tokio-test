use mio;
use mio::channel::channel;
use udp::UdpSocket;
use dns_query;
use dns_query::Message;
use transport::DnsTransport;

use futures::Future;

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

type DnsRequest = (u16, Message, CompleteMessage);
type CompleteMessage = Complete<Message, ()>;

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
        //println!("look up {}", host);
        let (c, v) = pair::<Message, ()>();
        let id = DNS_REQ_ID.fetch_add(1, Ordering::Relaxed) as u16;
        let msg = dns_query::build_query_message(id, dns_query::a_query(&host));
        let _ = self.tx.send((id, msg, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {

        if let Some((id, msg, c)) = self.rx.recv().unwrap() {            
            if let Ok(_) = self.transport.write(Frame::Message(msg)) {
                self.requests.insert(id, c);
            } else {
                // Can't requeue msg back because it's already been moved
                //let _ = self.tx.send((id, msg, c));                
            }
        }

        if let Ok(Some(Frame::Message(msg))) = self.transport.read() {                                
            if let Some(c) = self.requests.remove(&msg.get_id()) {
                c.complete(msg);
            }
        }

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
