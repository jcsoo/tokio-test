extern crate tokio;
extern crate futures;

use futures::Future;
use tokio::Service;
//use tokio::reactor::*;
use tokio::util::future::*;
//use std::io;

pub type Response = Val<usize, usize>;

struct HelloService {
    mult: usize,
}

impl Service for HelloService {
    type Req = usize;
    type Resp = usize;
    type Error = usize;
    type Fut = Response;

    fn call(&self, input: usize) -> Self::Fut {
        let value = input * self.mult;
        let (c,v) = pair();
        c.complete(value);
        v
    }

}

fn main() {
    let s = HelloService{mult: 2};
    println!("calling service");
    let f = s.call(42);
    f.then(move |res| {
        println!("result: {}", res.unwrap());
        res
    }).forget();

    //println!("done: {:?}", await(f));
}

fn await<T: Future>(f: T) -> Result<T::Item, T::Error> {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();

    f.then(move |res| {
        tx.send(res).unwrap();
        Ok::<(), ()>(())
    }).forget();

    rx.recv().unwrap()
}


/*
struct HelloWorld {
    rh: ReactorHandle,
}

impl Task for HelloWorld {
    fn tick(&mut self) -> io::Result<Tick> {
        println!("Hello, World!");
        self.rh.shutdown();
        Ok(Tick::Final)
    }
}

fn run_hello() {
    let r = Reactor::default().unwrap();
    let rh = r.handle();
    let t = HelloWorld{rh: rh.clone()};
    rh.schedule(t);
    r.run().unwrap();
}
*/
