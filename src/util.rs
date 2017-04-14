//! Utility code.

use hyper::StatusCode;
use hyper::header::ContentType;
use hyper::server::Response;


/// Create an erroneous JSON response.
pub fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_header(ContentType(mime!(Application/Json)))
        .with_body(json!({"error": message}).to_string())
}
