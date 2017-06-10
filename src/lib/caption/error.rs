//! Captioning error.

use std::error::Error;
use std::fmt;
use std::io;

use resources::{Loader, Font, FontLoader, Template, TemplateLoader};


/// Error that may occur during the captioning.
pub enum CaptionError<Tl = TemplateLoader, Fl = FontLoader>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    Template(String, Tl::Err),
    Font(String, Fl::Err),
    Encode(io::Error),
}

impl<Tl, Fl> Error for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn description(&self) -> &str { "captioning error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            CaptionError::Template(_, ref e) => Some(e),
            CaptionError::Font(_, ref e) => Some(e),
            CaptionError::Encode(ref e) => Some(e),
        }
    }
}

impl<Tl, Fl> fmt::Debug for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template(ref t, _) => write!(fmt, "CaptionError::Template({:?})", t),
            CaptionError::Font(ref f, _) => write!(fmt, "CaptionError::Font({:?})", f),
            CaptionError::Encode(ref e) => write!(fmt, "CaptionError::Encode({:?})", e)
        }
    }
}

impl<Tl, Fl> fmt::Display for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template(ref t, ref e) => write!(fmt, "cannot load template `{}`: {}", t, e),
            CaptionError::Font(ref f, ref e) => write!(fmt, "cannot load font `{}`: {}", f, e),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
        }
    }
}
