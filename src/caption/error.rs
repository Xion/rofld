//! Captioning error.

use std::error::Error;
use std::fmt;
use std::io;

use hyper::StatusCode;
use tokio_timer::{TimeoutError, TimerError};


/// Error that may occur during the captioning.
#[derive(Debug)]
pub enum CaptionError {
    // Errors related to rendering logic.
    Template(String),
    Font(String),
    Encode(io::Error),

    // Other.
    Timeout,
    Unavailable,
}
unsafe impl Send for CaptionError {}

impl CaptionError {
    #[inline]
    pub fn status_code(&self) -> StatusCode {
        match *self {
            CaptionError::Template(..) => StatusCode::NotFound,
            CaptionError::Font(..) => StatusCode::NotFound,
            CaptionError::Encode(..) => StatusCode::InternalServerError,
            CaptionError::Timeout => StatusCode::InternalServerError,
            CaptionError::Unavailable => StatusCode::ServiceUnavailable,
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
            CaptionError::Font(ref f) => write!(fmt, "cannot find font `{}`", f),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
            CaptionError::Timeout => write!(fmt, "caption task timed out"),
            CaptionError::Unavailable => write!(fmt, "captioning currently unavailable"),
        }
    }
}

// Necessary for imposing a timeout on the CaptionTask.
impl<F> From<TimeoutError<F>> for CaptionError {
    fn from(e: TimeoutError<F>) -> Self {
        match e {
            TimeoutError::Timer(_, TimerError::NoCapacity) => CaptionError::Unavailable,
            _ => CaptionError::Timeout,
        }
    }
}
