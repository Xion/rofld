//! Module with the server's request handlers.

mod captioner;
pub mod list;
pub mod util;


use std::env;
use std::error::Error;
use std::path::PathBuf;

use futures::{BoxFuture, future, Future};
use hyper::{self, Method, StatusCode, Uri};
use hyper::header::{ContentLength, ContentType};
use hyper::server::Response;
use rofl::{CaptionError, ImageMacro};
use serde_json;
use serde_qs;

pub use self::captioner::{CAPTIONER, RenderError};
use self::util::error_response;


lazy_static! {
    static ref TEMPLATE_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("templates");

    static ref FONT_DIR: PathBuf =
        env::current_dir().unwrap().join("data").join("fonts");
}


/// Handle the image captioning HTTP request.
pub fn caption_macro(method: Method, url: Uri, body: Vec<u8>) -> BoxFuture<Response, hyper::Error> {
    let parsed_im: Result<_, Box<Error>> = match method {
        Method::Get => {
            let query = match url.query() {
                Some(q) => { trace!("Caption request query string: {}", q); q }
                None => { trace!("No query string found in caption request"); "" }
            };
            debug!("Decoding image macro spec from {} bytes of query string",
                query.len());
            serde_qs::from_str(query).map_err(Into::into)
        },
        Method::Post => {
            trace!("Caption request body: {}", String::from_utf8_lossy(&body));
            debug!("Decoding image macro spec from {} bytes of JSON", body.len());
            serde_json::from_reader(&*body).map_err(Into::into)
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
                    format!("invalid format: {:?}", out.format)),
            };
            Response::new()
                .with_header(ContentType(mime_type))
                .with_header(ContentLength(out.bytes.len() as u64))
                .with_body(out.bytes)
        })
        .or_else(|e| future::ok(error_response(status_code_for(&e), e)))
        .boxed()
}


/// Determine the HTTP response code that best corresponds to a caption rendering error.
fn status_code_for(e: &RenderError) -> StatusCode {
    match *e {
        RenderError::Caption(ref e) => match *e {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Font(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
        },
        RenderError::Timeout => StatusCode::InternalServerError,
        RenderError::Unavailable => StatusCode::ServiceUnavailable,
    }
}
