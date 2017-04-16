//!
//! rofld  -- Lulz on demand
//!

             extern crate ansi_term;
             extern crate futures;
             extern crate futures_cpupool;
             extern crate glob;
             extern crate hyper;
             extern crate image;
             extern crate isatty;
#[macro_use] extern crate lazy_static;
             extern crate lru_cache;
#[macro_use] extern crate maplit;
#[macro_use] extern crate mime;
             extern crate num;
             extern crate rusttype;
             extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
             extern crate serde_qs;
             extern crate slog_envlogger;
             extern crate slog_stdlog;
             extern crate slog_stream;
             extern crate time;
#[macro_use] extern crate try_opt;

// `slog` must precede `log` in declarations here, because we want to simultaneously:
// * use the standard `log` macros (at least for a while)
// * be able to initialize the slog logger using slog macros like o!()
#[macro_use] extern crate slog;
#[macro_use] extern crate log;


mod ext;
mod caption;
mod logging;
mod text;
mod util;


use std::error::Error;

use futures::{future, Future};
use hyper::{Get, Post, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Http, Service, Request, Response};

use caption::{CAPTIONER, fonts, ImageMacro, templates};
use ext::futures::{ArcFuture, FutureExt};
use ext::hyper::BodyExt;
use util::error_response;


const HOST: &'static str = "0.0.0.0";
const PORT: u16 = 1337;


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
    type Future = ArcFuture<Self::Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // TODO: log the request after the response is served, in Common Log Format;
        // need to retain the request info first, and extract a handle() method
        // returning Response
        self.log(&req);

        match (req.method(), req.path()) {
            (_, "/caption") => return self.handle_caption(req),
            (&Get, "/templates") => return self.handle_list_templates(req),
            (&Get, "/fonts") => return self.handle_list_fonts(req),
            _ => {},
        }

        debug!("Path {} doesn't match any endpoint", req.path());
        let error_resp = match (req.method(), req.path()) {
            _ => Response::new().with_status(StatusCode::NotFound),
        };
        future::ok(error_resp).arc()
    }
}

// Request handlers.
impl Rofl {
    /// Handle the image captioning request.
    fn handle_caption(&self, request: Request) ->  <Self as Service>::Future {
        let (method, url, _, _, body) = request.deconstruct();
        body.into_bytes().and_then(move |bytes| {
            let parsed_im: Result<_, Box<Error>> = match method {
                Get => {
                    trace!("Decoding image macro spec from {} bytes of query string",
                        url.query().map(|q| q.len()).unwrap_or(0));
                    serde_qs::from_str(url.query().unwrap_or("")).map_err(Into::into)
                },
                Post => {
                    trace!("Decoding image macro spec from {} bytes of JSON", bytes.len());
                    serde_json::from_reader(&*bytes).map_err(Into::into)
                },
                _ => return future::ok(
                    Response::new().with_status(StatusCode::MethodNotAllowed)).arc(),
            };

            let im: ImageMacro = match parsed_im {
                Ok(im) => im,
                Err(e) => {
                    error!("Failed to decode image macro: {}", e);
                    return future::ok(error_response(
                        StatusCode::BadRequest, "cannot decode request")).arc();
                },
            };
            debug!("Decoded {:?}", im);

            CAPTIONER.render(im)
                .map(|image_bytes| {
                    Response::new()
                        .with_header(ContentType(mime!(Image/Png)))
                        .with_body(image_bytes)
                })
                .or_else(|e| future::ok(e.into()))
                .arc()
        })
        .arc()
    }

    /// Handle the template listing request.
    fn handle_list_templates(&self, _: Request) -> <Self as Service>::Future {
        let template_names = templates::list();
        let response = Response::new()
            .with_body(json!(template_names).to_string());
        future::ok(response).arc()
    }

    /// Handle the font listing request.
    fn handle_list_fonts(&self, _: Request) -> <Self as Service>::Future {
        let font_names = fonts::list();
        let response = Response::new()
            .with_body(json!(font_names).to_string());
        future::ok(response).arc()
    }
}

impl Rofl {
    #[inline]
    fn log(&self, req: &Request) {
        info!("{} {} {}{} {}",
            req.remote_addr().map(|a| format!("{}", a.ip())).unwrap_or_else(|| "-".to_owned()),
            format!("{}", req.method()).to_uppercase(),
            req.path(),
            req.query().map(|q| format!("?{}", q)).unwrap_or_else(String::new),
            req.version());
    }
}
