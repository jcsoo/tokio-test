//! A generic Tokio TCP client implementation.

use udp::UdpSocket;
use tokio::reactor::{self, ReactorHandle, Task};
use std::net::SocketAddr;
use std::io;


pub fn connect<T>(reactor: &ReactorHandle, addr: SocketAddr, new_task: T)
        where T: NewTask
{
    reactor.oneshot(move || {
        // Create a new Tokio TcpStream from the Mio socket
        let socket = match UdpSocket::bind(&addr) {
            Ok(s) => s,
            Err(_) => unimplemented!(),
        };

        let task = match new_task.new_task(socket) {
            Ok(d) => d,
            Err(_) => unimplemented!(),
        };

        try!(reactor::schedule(task));
        Ok(())
    });
}

/// Creates new `Task` values
pub trait NewTask: Send + 'static {
    /// The `Task` value created by this factory
    type Item: Task;

    /// Create and return a new `Task` value
    fn new_task(&self, stream: UdpSocket) -> io::Result<Self::Item>;
}
