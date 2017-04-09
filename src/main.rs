//!
//! rofld  -- Lulz on demand
//!

extern crate futures;
extern crate hyper;


use futures::future::{self, FutureResult};
use hyper::{Get, StatusCode};
use hyper::server::{Http, Service, Request, Response};


const HOST: &'static str = "0.0.0.0";
const PORT: u16 = 1337;


fn main() {
    let addr = format!("{}:{}", HOST, PORT).parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(Rofl)).unwrap();
    server.run().unwrap();
}


/// Hyper async service implementing all the functionality.
pub struct Rofl;

impl Service for Rofl {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let result = match (req.method(), req.path()) {
            (&Get, "/") => Response::new().with_body("Hello world".as_bytes()),
            _ => Response::new().with_status(StatusCode::NotFound),
        };
        future::ok(result)
    }
}
