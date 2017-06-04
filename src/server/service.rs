//! Module with the service that implements ALL the functionality.

use std::hash::Hash;
use std::time::{Duration, SystemTime};

use futures::{BoxFuture, future, Future};
use hyper::{self, Get, StatusCode};
use hyper::header::{Expires, ContentLength, ContentType};
use hyper::server::{Service, Request, Response};
use rofl::ThreadSafeCache;
use serde_json::Value as Json;
use time::precise_time_s;

use ext::hyper::BodyExt;
use handlers::{CAPTIONER, caption_macro};
use handlers::list::{list_fonts, list_templates};
use handlers::util::json_response;


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

        let start = precise_time_s();
        self.handle(req).map(move |mut resp| {
            Self::fix_headers(&mut resp);

            let finish = precise_time_s();
            debug!("HTTP {status}, produced {len} bytes of {ctype} in {time:.3} secs",
                status = resp.status(),
                len = if resp.headers().has::<ContentLength>() {
                    format!("{}", **resp.headers().get::<ContentLength>().unwrap())
                } else {
                    "unknown number of".into()
                },
                ctype = resp.headers().get::<ContentType>().unwrap(),
                time = finish - start);
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
        body.into_bytes()
            .and_then(move |body| caption_macro(method, url, body))
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
                "templates": cache_stats(CAPTIONER.template_cache()),
                "fonts": cache_stats(CAPTIONER.font_cache()),
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
