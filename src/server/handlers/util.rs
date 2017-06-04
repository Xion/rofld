//! Utilities for request handlers.

use hyper::StatusCode;
use hyper::header::{ContentLength, ContentType};
use hyper::server::Response;
use serde_json::{self, Value as Json};


/// Create a JSON response.
pub fn json_response(json: Json) -> Response {
    let body = json.to_string();
    Response::new()
        .with_header(ContentType(mime!(Application/Json)))
        .with_header(ContentLength(body.len() as u64))
        .with_body(body)
}

/// Create an erroneous JSON response.
pub fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    json_response(json!({"error": message.to_string()}))
        .with_status(status_code)
}
