//!
//! rofld  -- Lulz on demand
//!

extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;


mod ext;


use futures::Future;
use futures::future::{self, BoxFuture};
use hyper::{Get, Post, StatusCode};
use hyper::server::{Http, Service, Request, Response};
use serde_json::Value as Json;

use ext::hyper::BodyExt;


const HOST: &'static str = "0.0.0.0";
const PORT: u16 = 1337;


fn main() {
    let addr = format!("{}:{}", HOST, PORT).parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(Rofl)).unwrap();
    server.run().unwrap();
}


/// Hyper async service implementing ALL the functionality.
pub struct Rofl;

impl Service for Rofl {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/") => box_ok(Response::new().with_body("Hello world")),
            (&Post, "/") => {
                req.body().into_bytes().map(|bytes| {
                    let json: Json = match serde_json::from_reader(&*bytes) {
                        Ok(json) => json,
                        Err(_) => return Response::new().with_status(StatusCode::BadRequest),
                    };
                    match json.pointer("/template").and_then(|t| t.as_str()) {
                        Some(template) => Response::new().with_body(template.to_owned()),
                        None => Response::new()
                            .with_status(StatusCode::BadRequest)
                            .with_body("git gud"),
                    }
                }).boxed()
            },
            _ => box_ok(Response::new().with_status(StatusCode::NotFound)),
        }
    }
}


pub fn box_ok<T, E>(t: T) -> BoxFuture<T, E>
    where T: Send + 'static, E: Send + 'static
{
    future::ok(t).boxed()
}
