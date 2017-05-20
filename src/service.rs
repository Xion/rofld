//! Module with the service that implements ALL the functionality.

use std::error::Error;
use std::hash::Hash;
use std::time::{Duration, SystemTime};

use futures::{BoxFuture, future, Future};
use hyper::{self, Get, Post, StatusCode};
use hyper::header::{Expires, ContentLength, ContentType};
use hyper::server::{Service, Request, Response};
use serde_json::{self, Value as Json};
use serde_qs;

use caption::CAPTIONER;
use ext::hyper::BodyExt;
use model::ImageMacro;
use resources::{list_fonts, list_templates, ThreadSafeCache};


pub struct Rofl;

impl Service for Rofl {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Request) -> Self::Future {
        // TODO: log the request after the response is served, in Common Log Format;
        // need to retain the request info first
        self.log(&req);
        self.handle(req).map(|mut resp| {
            Self::fix_headers(&mut resp);
            debug!("HTTP {}, sent {} bytes (ContentType: {})",
                resp.status(),
                if resp.headers().has::<ContentLength>() {
                    format!("{}", **resp.headers().get::<ContentLength>().unwrap())
                } else {
                    "unknown number of".into()
                },
                resp.headers().get::<ContentType>().unwrap());
            resp
        }).boxed()
    }
}
impl Rofl {
    fn handle(&self, req: Request) -> <Rofl as Service>::Future {
        match (req.method(), req.path()) {
            (_, "/caption") => self.handle_caption(req),
            (&Get, "/templates") => self.handle_list_templates(req),
            (&Get, "/fonts") => self.handle_list_fonts(req),
            (&Get, "/stats") => self.handle_stats(req),
            _ => self.handle_404(req),
        }
    }

    fn handle_404(&self, req: Request) -> <Rofl as Service>::Future {
        debug!("Path {} doesn't match any endpoint", req.path());
        let response = Response::new().with_status(StatusCode::NotFound)
            .with_header(ContentType::plaintext())
            .with_header(ContentLength(0));
        future::ok(response).boxed()
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
                    let query = match url.query() {
                        Some(q) => { trace!("Caption request query string: {}", q); q }
                        None => { trace!("No query string found in caption request"); "" }
                    };
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
                    let response = Response::new().with_status(StatusCode::MethodNotAllowed)
                        .with_header(ContentType::plaintext())
                        .with_header(ContentLength(0));
                    return future::ok(response).boxed();
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
                .map(|out| {
                    let mime_type = match out.mime_type() {
                        Some(mt) => mt,
                        None => return error_response(
                            StatusCode::InternalServerError,
                            format!("invalid template format: {:?}", out.format)),
                    };
                    Response::new()
                        .with_header(ContentType(mime_type))
                        .with_header(ContentLength(out.bytes.len() as u64))
                        .with_body(out.bytes)
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
        let response = json_response(json!(template_names));
        future::ok(response).boxed()
    }

    /// Handle the font listing request.
    fn handle_list_fonts(&self, _: Request) -> <Self as Service>::Future {
        let font_names = list_fonts();
        let response = json_response(json!(font_names));
        future::ok(response).boxed()
    }

    /// Handle the server statistics request.
    fn handle_stats(&self, _: Request) -> <Self as Service>::Future {
        let stats = json!({
            "cache": {
                "templates": cache_stats(CAPTIONER.cache().templates()),
                "fonts": cache_stats(CAPTIONER.cache().fonts()),
            }
        });
        return future::ok(json_response(stats)).boxed();

        fn cache_stats<K: Eq + Hash, V>(cache: &ThreadSafeCache<K, V>) -> Json {
            let capacity = cache.capacity();
            json!({
                "capacity": capacity,
                "fill_rate": cache.len() as f32 / capacity as f32,
                "misses": cache.misses(),
                "hits": cache.hits(),
            })
        }
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

    /// Fix headers in the response, providing default values where necessary.
    fn fix_headers(resp: &mut Response) {
        if !resp.headers().has::<ContentType>() {
            resp.headers_mut().set(ContentType::octet_stream());
        }
        if !resp.headers().has::<Expires>() {
            let century = Duration::from_secs(100 * 365 * 24 * 60 * 60);
            let far_future = SystemTime::now() + century;
            resp.headers_mut().set(Expires(far_future.into()));
        }
    }
}


// Utility functions

/// Create a JSON response.
fn json_response(json: Json) -> Response {
    let body = json.to_string();
    Response::new()
        .with_header(ContentType(mime!(Application/Json)))
        .with_header(ContentLength(body.len() as u64))
        .with_body(body)
}

/// Create an erroneous JSON response.
fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    json_response(json!({"error": message.to_string()}))
        .with_status(status_code)
}
