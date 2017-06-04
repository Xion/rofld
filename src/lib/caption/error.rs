//! Captioning error.

use std::error::Error;
use std::fmt;
use std::io;


/// Error that may occur during the captioning.
#[derive(Debug)]
pub enum CaptionError {
    Template(String),
    Font(String),
    Encode(io::Error),
}
unsafe impl Send for CaptionError {}

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
        }
    }
}
