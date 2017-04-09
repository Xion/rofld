//!
//! rofld  -- Lulz on demand
//!

             extern crate futures;
             extern crate hyper;
             extern crate serde;
#[macro_use] extern crate serde_derive;
             extern crate serde_json;


mod ext;


use futures::Future;
use futures::future::{self, BoxFuture};
use hyper::{Get, Post, StatusCode};
use hyper::server::{Http, Service, Request, Response};

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
            (&Get, "/") => box_ok(Response::new()
                .with_status(StatusCode::MethodNotAllowed)),
            (&Post, "/") => {
                req.body().into_bytes().map(|bytes| {
                    match serde_json::from_reader::<_, ImageMacro>(&*bytes) {
                        Ok(im) => Response::new().with_body(im.template),
                        Err(_) => Response::new()
                            .with_status(StatusCode::BadRequest)
                            .with_body("git gud"),
                    }
                }).boxed()
            },
            _ => box_ok(Response::new().with_status(StatusCode::NotFound)),
        }
    }
}

/// Describes an image macro, used as an input structure.
#[derive(Deserialize)]
struct ImageMacro {
    template: String,
    top_text: Option<String>,
    middle_text: Option<String>,
    bottom_text: Option<String>,
}


pub fn box_ok<T, E>(t: T) -> BoxFuture<T, E>
    where T: Send + 'static, E: Send + 'static
{
    future::ok(t).boxed()
}
