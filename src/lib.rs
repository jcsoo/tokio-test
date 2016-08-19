#![allow(dead_code)]

extern crate tokio;
extern crate futures;
extern crate mio;
extern crate trust_dns;

mod udp;

mod transport;
pub mod dns_query;
pub mod resolver;
