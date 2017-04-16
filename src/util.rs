//! Utility code.

use hyper::StatusCode;
use hyper::header::ContentType;
use hyper::server::Response;


/// Derive an implementation of From<InnerType> fpr one variant of a unary enum.
/// Adapter from the source of error_derive crate.
macro_rules! derive_enum_from(
    ($inner:ty => $enum_:ident::$variant:ident) => {
        impl ::std::convert::From<$inner> for $enum_ {
            fn from(v: $inner) -> $enum_ {
                $enum_::$variant(v)
            }
        }
    }
);

/// Converts a value to a "static" (though not &'static) str
/// so it cam be used with APIs that only accept borrowed strings.
macro_rules! to_static_str(
    ($v:expr) => ({
        lazy_static! {
            static ref DUMMY: String = $v.to_string();
        }
        &*DUMMY as &str
    })
);


/// Create an erroneous JSON response.
pub fn error_response<T: ToString>(status_code: StatusCode, message: T) -> Response {
    let message = message.to_string();
    Response::new()
        .with_status(status_code)
        .with_header(ContentType(mime!(Application/Json)))
        .with_body(json!({"error": message}).to_string())
}
