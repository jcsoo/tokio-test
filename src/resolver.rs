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
use std::io;

type CompleteMessage = Complete<Message, ()>;

struct Resolver {
    transport: DnsTransport,    
    tx: mio::channel::Sender<(String, CompleteMessage)>,
    rx: Receiver<(String, CompleteMessage)>,
    requests: HashMap<u16, CompleteMessage>,
    next_id: u16,
}

pub struct ResolverHandle {
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
    pub fn lookup(&self, host: String) -> Box<Future<Item = Message, Error = ()>> {
        println!("look up {}", host);
        let (c, v) = pair::<Message, ()>();
        let _ = self.tx.send((host, c));
        Box::new(v)
    }
}

impl Task for Resolver {
    fn tick(&mut self) -> io::Result<Tick> {                
        while self.rx.is_readable() {            
            if let Some((host, c)) = self.rx.recv().unwrap() {
                let id = self.next_id();
                let msg = dns_query::build_query_message(id, dns_query::a_query(&host));
                let frame = Frame::Message(msg);
            
                if let Ok(_) = self.transport.write(frame) {
                    self.requests.insert(id, c);
                } else {
                    let _ = self.tx.send((host, c));
                    break;
                }
            } else {
                break;
            }
        }
        
        while self.transport.is_readable() {                        
            if let Ok(Some(Frame::Message(msg))) = self.transport.read() {                
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

pub fn resolver(addr: SocketAddr) -> ResolverHandle {
    // must be run within a reactor
    let (tx, rx) = channel::<(String, CompleteMessage)>();
    let socket = UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
    let transport = DnsTransport::new(socket, addr);        
    {
        let r = Resolver {
            transport: transport,            
            tx: tx.clone(),
            rx: Receiver::watch(rx).unwrap(),
            requests: HashMap::new(),
            next_id: 1000,
        };
        reactor::schedule(r).unwrap();
    }
    
    ResolverHandle { tx: tx }
}
