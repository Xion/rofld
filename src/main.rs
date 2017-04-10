//!
//! rofld  -- Lulz on demand
//!

             extern crate ansi_term;
             extern crate futures;
             extern crate glob;
             extern crate hyper;
             extern crate image;
             extern crate isatty;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate maplit;
             extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
             extern crate slog_envlogger;
             extern crate slog_stdlog;
             extern crate slog_stream;
             extern crate time;

// `slog` must precede `log` in declarations here, because we want to simultaneously:
// * use the standard `log` macros (at least for a while)
// * be able to initialize the slog logger using slog macros like o!()
#[macro_use] extern crate slog;
#[macro_use] extern crate log;


mod ext;
mod logging;


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
    // TODO: logging verbosity command line flag
    logging::init(1).unwrap();

    let addr = format!("{}:{}", HOST, PORT).parse().unwrap();
    info!("Starting server to listen on {}...", addr);
    let server = Http::new().bind(&addr, || Ok(Rofl)).unwrap();

    debug!("Entering event loop...");
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
            (&Post, "/caption") => return self.handle_caption(req),
            (&Get, "/templates") => return self.handle_list_templates(req),
            _ => {},
        }

        let error_resp = match (req.method(), req.path()) {
            (&Get, "/") => Response::new().with_status(StatusCode::MethodNotAllowed),
            _ => Response::new().with_status(StatusCode::NotFound),
        };
        future::ok(error_resp).boxed()
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
            match im.render(&mut image) {
                Ok(_) => Response::new().with_body(image),
                Err(e) => e.into(),
            }
        }).boxed()
    }

    /// Handle the template listing request.
    fn handle_list_templates(&self, _: Request) -> <Self as Service>::Future {
        let templates =
            glob::glob(&format!("{}", TEMPLATE_DIR.join("*.*").display())).unwrap()
                .filter_map(Result::ok)
                .fold(vec![], |mut ts, t| {
                    let name = t.file_stem().unwrap().to_str().unwrap().to_owned();
                    ts.push(name); ts
                });
        let response = Response::new()
            .with_body(json!(templates).to_string());
        future::ok(response).boxed()
    }
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
    pub fn render<W: Write>(&self, writer: &mut W) -> Result<(), CaptionError> {
        // TODO: cache templates
        let template_glob = &format!(
            "{}", TEMPLATE_DIR.join(self.template.clone() + ".*").display());
        let mut template_iter = match glob::glob(template_glob) {
            Ok(it) => it,
            Err(e) => {
                error!("Failed to glob over template files: {}", e);
                return Err(CaptionError::Template(self.template.clone()));
            },
        };
        let template_path = template_iter.next().and_then(|p| p.ok())
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
impl CaptionError {
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        match *self {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
        }
    }
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
impl Into<Response> for CaptionError {
    fn into(self) -> Response {
        error_response(self.status_code(), format!("{}", self))
    }
}


fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_body(json!({"error": message}).to_string())
}
