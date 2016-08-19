use dns_query::{encode_message, decode_message, Message};
use udp::UdpSocket;

use tokio::proto::pipeline::Frame;
use tokio::io::{Readiness, Transport};

use std::net::SocketAddr;
use std::io;


pub type MsgFrame = Frame<Message, io::Error>;

pub struct DnsTransport {
    inner: UdpSocket,
    addr: SocketAddr,
    buf: Vec<u8>,
}

impl DnsTransport
{
    pub fn new(inner: UdpSocket, addr: SocketAddr) -> DnsTransport {
        DnsTransport {
            inner: inner,
            addr: addr,
            buf: Vec::with_capacity(4096),
        }
    }
}

impl Transport for DnsTransport
{
    type In = MsgFrame;
    type Out = MsgFrame;

    fn read(&mut self) -> io::Result<Option<MsgFrame>> {
        let mut buf = [0u8; 512];
        match self.inner.recv_from(&mut buf) {
            Ok((_, _)) => Ok(Some(Frame::Message(decode_message(&mut buf)))),
            Err(e) => Err(e),
        }
    }
    
    fn write(&mut self, msg: MsgFrame) -> io::Result<Option<()>> {
        if let Frame::Message(message) = msg {
            encode_message(&mut self.buf, message);
            let _ = try!(self.inner.send_to(&self.buf, &self.addr));
            self.buf.clear();
            Ok(Some(()))
        } else {
            panic!("unexpected frame type")
        }
    }

    fn flush(&mut self) -> io::Result<Option<()>> {
        Ok(Some(()))
    }
}

impl Readiness for DnsTransport
{
    fn is_readable(&self) -> bool {
        self.inner.is_readable()
    }

    fn is_writable(&self) -> bool {
        self.inner.is_writable()
    }
}