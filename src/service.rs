//! Module with the service that implements ALL the functionality.

use std::error::Error;

use futures::{BoxFuture, future, Future};
use hyper::{self, Get, Post, StatusCode};
use hyper::header::ContentType;
use hyper::server::{Service, Request, Response};
use serde_json;
use serde_qs;

use caption::CAPTIONER;
use ext::hyper::BodyExt;
use model::ImageMacro;
use resources::{list_fonts, list_templates};


pub struct Rofl;

impl Service for Rofl {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // TODO: log the request after the response is served, in Common Log Format;
        // need to retain the request info first, and extract a handle() method
        // returning Response
        self.log(&req);

        match (req.method(), req.path()) {
            (_, "/caption") => return self.handle_caption(req),
            (&Get, "/templates") => return self.handle_list_templates(req),
            (&Get, "/fonts") => return self.handle_list_fonts(req),
            (&Get, "/stats") => return self.handle_stats(req),
            _ => {},
        }

        debug!("Path {} doesn't match any endpoint", req.path());
        let error_resp = match (req.method(), req.path()) {
            _ => Response::new().with_status(StatusCode::NotFound),
        };
        future::ok(error_resp).boxed()
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
                    let query = url.query().unwrap_or("");
                    trace!("Caption request query string: {}", query);
                    debug!("Decoding image macro spec from {} bytes of query string",
                        query.len());
                    serde_qs::from_str(query).map_err(Into::into)
                },
                Post => {
                    trace!("Caption request body: {}", String::from_utf8_lossy(&bytes));
                    debug!("Decoding image macro spec from {} bytes of JSON", bytes.len());
                    serde_json::from_reader(&*bytes).map_err(Into::into)
                },
                m => {
                    warn!("Unsupported HTTP method for caption request: {}", m);
                    return future::ok(
                        Response::new().with_status(StatusCode::MethodNotAllowed)).boxed();
                },
            };

            let im: ImageMacro = match parsed_im {
                Ok(im) => im,
                Err(e) => {
                    error!("Failed to decode image macro: {}", e);
                    return future::ok(error_response(
                        StatusCode::BadRequest,
                        format!("cannot decode request: {}", e))).boxed();
                },
            };
            debug!("Decoded {:?}", im);

            CAPTIONER.render(im)
                .map(|image_bytes| {
                    Response::new()
                        .with_header(ContentType(mime!(Image/Png)))
                        .with_body(image_bytes)
                })
                .or_else(|e| future::ok(
                    error_response(e.status_code(), format!("{}", e))
                ))
                .boxed()
        })
        .boxed()
    }

    /// Handle the template listing request.
    fn handle_list_templates(&self, _: Request) -> <Self as Service>::Future {
        let template_names = list_templates();
        let response = Response::new()
            .with_body(json!(template_names).to_string());
        future::ok(response).boxed()
    }

    /// Handle the font listing request.
    fn handle_list_fonts(&self, _: Request) -> <Self as Service>::Future {
        let font_names = list_fonts();
        let response = Response::new()
            .with_body(json!(font_names).to_string());
        future::ok(response).boxed()
    }

    /// Handle the server statistics request.
    fn handle_stats(&self, _: Request) -> <Self as Service>::Future {
        let template_capacity = CAPTIONER.cache().templates().capacity();
        let font_capacity = CAPTIONER.cache().fonts().capacity();

        let stats = json!({
            "cache": {
                "templates": {
                    "capacity": template_capacity,
                    "fill_rate": CAPTIONER.cache().templates().len() as f32 / template_capacity as f32,
                    "misses": CAPTIONER.cache().templates().misses(),
                    "hits": CAPTIONER.cache().templates().hits(),
                },
                "fonts": {
                    "capacity": font_capacity,
                    "fill_rate": CAPTIONER.cache().fonts().len() as f32 / font_capacity as f32,
                    "misses": CAPTIONER.cache().fonts().misses(),
                    "hits": CAPTIONER.cache().fonts().hits(),
                }
            }
        });
        future::ok(Response::new().with_body(stats.to_string())).boxed()
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


// Utility functions

/// Create an erroneous JSON response.
fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_header(ContentType(mime!(Application/Json)))
        .with_body(json!({"error": message}).to_string())
}
