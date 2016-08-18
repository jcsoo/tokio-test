#![allow(dead_code)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;
mod udp_client;
mod dns_query;
mod resolver;

use dns_query::Message;

use futures::Future;

use tokio::reactor::{self, Reactor};

fn print_response(m: Message) {
    println!("msg: {:?}", m.get_answers());
}

fn main() {
    let reactor = Reactor::default().unwrap();
    reactor.handle().oneshot(move || {
        let resolver = resolver::resolver("8.8.8.8:53".parse().unwrap());
        let r1 = resolver.lookup("google.com".to_owned())
            .map(print_response);

        let r2 = resolver.lookup("amazon.com".to_owned())
            .map(print_response);

        let r3 = resolver.lookup("apple.com".to_owned())
            .map(print_response);

        r1.join(r2)
            .join(r3)
            .map(|_| {
                reactor::shutdown().unwrap();
            })
            .forget();
    });
    reactor.run().unwrap();
}
