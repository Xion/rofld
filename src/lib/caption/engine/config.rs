//! Module with captioning engine configuration.

use std::error;
use std::fmt;


/// Structure holding configuration for the `Engine`.
///
/// This is shared with `CaptionTask`s.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Quality of the generated JPEG images (in %).
    pub jpeg_quality: u8,
    /// Quality of the generated GIF animations (in %).
    pub gif_quality: u8,
}

impl Default for Config {
    /// Initialize Config with default values.
    fn default() -> Self {
        Config {
            jpeg_quality: 85,
            gif_quality: 60,
        }
    }
}


/// Error signifying an invalid value for one of the configuration options.
#[derive(Clone, Debug)]
pub enum Error {
    /// Invalid value for the GIF animation quality percentage.
    GifQuality(u8),
    /// Invalid value for the JPEG image quality percentage.
    JpegQuality(u8),
}

impl error::Error for Error {
    fn description(&self) -> &str { "invalid Engine configuration value" }
    fn cause(&self) -> Option<&error::Error> { None }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::GifQuality(q) => write!(fmt, "invalid GIF quality value: {}%", q),
            Error::JpegQuality(q) => write!(fmt, "invalid JPEG quality value: {}%", q),
        }
    }
}
