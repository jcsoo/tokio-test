
use tokio::reactor::{Reactor, Task, Tick};
use tokio::io::TryRead;
use tokio::tcp::TcpStream;
use tokio::server;
use std::io;

struct Connection {
    stream: TcpStream,
    buf: Box<[u8]>,
}

impl Connection {
    fn new(stream: TcpStream) -> Connection {
        let buf = vec![0; 1024];
        println!("new connection");

        Connection {
            stream: stream,
            buf: buf.into_boxed_slice(),
        }
    }
}

impl Task for Connection {
    fn tick(&mut self) -> io::Result<Tick> {
        while let Some(n) = try!(self.stream.try_read(&mut self.buf)) {
            println!("read {} bytes", n);

            if n == 0 {
                // Socket closed, shutdown
                return Ok(Tick::Final);
            }
        }

        Ok(Tick::WouldBlock)
    }
}

pub fn main() {
    println!("running");

    let reactor = Reactor::default().unwrap();
    server::listen(&reactor.handle(), "0.0.0.0:5000".parse().unwrap(),
        |stream| Ok(Connection::new(stream))
    ).unwrap();
    reactor.run().unwrap();

    println!("complete");
}
