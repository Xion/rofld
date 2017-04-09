//!
//! rofld  -- Lulz on demand
//!

             extern crate futures;
             extern crate glob;
             extern crate hyper;
             extern crate image;
#[macro_use] extern crate lazy_static;
             extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;


mod ext;


use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use futures::Future;
use futures::future::{self, BoxFuture};
use hyper::{Get, Post, StatusCode};
use hyper::server::{Http, Service, Request, Response};
use image::GenericImage;

use ext::hyper::BodyExt;


const HOST: &'static str = "0.0.0.0";
const PORT: u16 = 1337;

lazy_static! {
    static ref TEMPLATE_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("templates");
}


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
            (&Post, "/") => self.handle_caption(req),
            _ => box_ok(Response::new().with_status(StatusCode::NotFound)),
        }
    }
}

impl Rofl {
    /// Handle the image captioning request.
    fn handle_caption(&self, request: Request) ->  <Self as Service>::Future {
        request.body().into_bytes().map(|bytes| {
            let im: ImageMacro = match serde_json::from_reader(&*bytes) {
                Ok(im) => im,
                Err(_) => return error_response(
                    StatusCode::BadRequest, "cannot decode JSON request"),
            };
            let mut image = vec![];
            match im.generate(&mut image) {
                Ok(_) => Response::new().with_body(image),
                Err(e) => error_response(StatusCode::BadRequest, format!("{}", e)),
            }
        }).boxed()
    }
}

fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_body(json!({"error": message}).to_string())
}


/// Describes an image macro, used as an input structure.
#[derive(Deserialize)]
struct ImageMacro {
    template: String,
    width: u32,
    height: u32,
    top_text: Option<String>,
    middle_text: Option<String>,
    bottom_text: Option<String>,
}

impl ImageMacro {
    pub fn generate<W: Write>(&self, writer: &mut W) -> Result<(), CaptionError> {
        // TODO: cache templates
        let template_path =
            glob::glob(&format!("{}/{}.*", TEMPLATE_DIR.display(), self.template)).unwrap()
                .next().and_then(|p| p.ok())
                .ok_or_else(|| CaptionError::Template(self.template.clone()))?;

        let img = image::open(template_path)
            .map_err(|_| CaptionError::Template(self.template.clone()))?;

        // TODO: render the text
        let img = img.resize(self.width, self.height, image::FilterType::Lanczos3);
        let (width, height) = img.dimensions();
        image::png::PNGEncoder::new(writer)
            .encode(&*img.raw_pixels(), width, height, image::ColorType::RGB(8))
            .map_err(CaptionError::Encode)
    }
}

/// Error that may occur during the captioning.
#[derive(Debug)]
enum CaptionError {
    Template(String),
    Encode(io::Error),
}
impl Error for CaptionError {
    fn description(&self) -> &str { "captioning error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            CaptionError::Encode(ref e) => Some(e),
            _ => None,
        }
    }
}
impl fmt::Display for CaptionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template(ref t) => write!(fmt, "cannot find template `{}`", t),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
        }
    }
}


pub fn box_ok<T, E>(t: T) -> BoxFuture<T, E>
    where T: Send + 'static, E: Send + 'static
{
    future::ok(t).boxed()
}
