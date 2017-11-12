//! Data structures for command-line arguments.

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use clap;
use rofl::ImageMacro;
use serde_json;

use super::image_macro::Error as ImageMacroError;


/// Structure to hold options received from the command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// Verbosity of the logging output.
    ///
    /// Corresponds to the number of times the -v flag has been passed.
    /// If -q has been used instead, this will be negative.
    pub verbosity: isize,

    /// The image macro to create.
    pub image_macro: ImageMacro,
    /// Path to write the finished image macro to.
    ///
    /// If absent, it shall be written to standard output.
    pub output_path: Option<PathBuf>,
}

#[allow(dead_code)]
impl Options {
    #[inline]
    pub fn verbose(&self) -> bool { self.verbosity > 0 }
    #[inline]
    pub fn quiet(&self) -> bool { self.verbosity < 0 }
}


macro_attr! {
    /// Error that can occur while parsing of command line arguments.
    #[derive(Debug, EnumFromInner!)]
    pub enum ArgsError {
        /// General when parsing the arguments.
        Parse(clap::Error),
        /// Image macro argument syntax error.
        ImageMacroArg(ImageMacroError),
        /// Image macro --json parsing error.
        ImageMacroJson(serde_json::Error),
    }
}

impl Error for ArgsError {
    fn description(&self) -> &str { "command line argument error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            ArgsError::Parse(ref e) => Some(e),
            ArgsError::ImageMacroArg(ref e) => Some(e),
            ArgsError::ImageMacroJson(ref e) => Some(e),
        }
    }
}

impl fmt::Display for ArgsError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArgsError::Parse(ref e) => write!(fmt, "invalid arguments: {}", e),
            ArgsError::ImageMacroArg(ref e) => {
                write!(fmt, "image macro argument syntax error: {}", e)
            }
            ArgsError::ImageMacroJson(ref e) => {
                write!(fmt, "image macro JSON error: {}", e)
            }
        }
    }
}
