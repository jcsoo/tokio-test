use mio;
use mio::channel::channel;
use udp::UdpSocket;
use dns_query;
use dns_query::Message;
use transport::DnsTransport;

use futures::Future;

use tokio::io::{Transport, Readiness};
use tokio::reactor::{self, Task, Tick};
use tokio::util::future::{pair, Complete};
use tokio::util::channel::Receiver;
use tokio::proto::pipeline::Frame;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use std::io;

static DNS_REQ_ID: AtomicUsize = ATOMIC_USIZE_INIT;

type CompleteMessage = Complete<Message, ()>;

struct Resolver {
    transport: DnsTransport,    
    tx: mio::channel::Sender<(u16, String, CompleteMessage)>,
    rx: Receiver<(u16, String, CompleteMessage)>,
    requests: HashMap<u16, CompleteMessage>,    
}

pub struct ResolverHandle {
    tx: mio::channel::Sender<(u16, String, CompleteMessage)>,
}

impl Resolver {

}

impl ResolverHandle {
    pub fn lookup(&self, host: String) -> Box<Future<Item = Message, Error = ()>> {
        println!("look up {}", host);
        let (c, v) = pair::<Message, ()>();
        let id = DNS_REQ_ID.fetch_add(1, Ordering::Relaxed) as u16;
        let _ = self.tx.send((id, host, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {                
        while self.rx.is_readable() {            
            if let Some((id, host, c)) = self.rx.recv().unwrap() {
                let msg = dns_query::build_query_message(id, dns_query::a_query(&host));                
            
                if let Ok(_) = self.transport.write(Frame::Message(msg)) {
                    self.requests.insert(id, c);
                } else {
                    let _ = self.tx.send((id, host, c));
                    break;
                }
            } else {
                break;
            }
        }
        
        while self.transport.is_readable() {                        
            if let Ok(Some(Frame::Message(msg))) = self.transport.read() {                                
                if let Some(c) = self.requests.remove(&msg.get_id()) {
                    c.complete(msg);
                }
            } else {
                break;
            }
        }

        Ok(Tick::WouldBlock)
    }
}

pub fn resolver(addr: SocketAddr) -> ResolverHandle {
    // must be run within a reactor
    let (tx, rx) = channel::<(u16, String, CompleteMessage)>();
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
