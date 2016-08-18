use mio;
use mio::channel::channel;
use udp::UdpSocket;
use dns_query;
use dns_query::Message;

use futures::Future;

use tokio::io::Readiness;
use tokio::reactor::{self, Task, Tick};
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
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let socket = match self.socket.take() {
            Some(s) => s,
            None => UdpSocket::bind(&"0.0.0.0:0".parse().unwrap()).unwrap(),
        };

        // Avoid short circuit evaluation of both sources 

        let socket_writable = socket.is_writable();
        let rx_readable = self.rx.is_readable();

        while socket_writable && rx_readable {
            buf.clear();
            if let Some((host, c)) = self.rx.recv().unwrap() {
                let id = self.next_id();
                dns_query::encode_query(&mut buf, id, dns_query::a_query(&host));
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
                let msg = dns_query::decode_message(&mut buf);
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

pub fn resolver(addr: SocketAddr) -> ResolverHandle {
    // must be run within a reactor
    let (tx, rx) = channel::<(String, CompleteMessage)>();

    {
        let r = Resolver {
            socket: None,
            addr: addr,
            tx: tx.clone(),
            rx: Receiver::watch(rx).unwrap(),
            requests: HashMap::new(),
            next_id: 1000,
        };
        reactor::schedule(r).unwrap();
    }
    
    ResolverHandle { tx: tx }
}
