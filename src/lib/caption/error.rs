//! Captioning error.

use std::error::Error;
use std::fmt;
use std::io;

use resources::{Loader, Font, FontLoader, Template, TemplateLoader};


/// Error that may occur during the captioning.
pub enum CaptionError<Tl = TemplateLoader, Fl = FontLoader>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Error while loading the template.
    Template {
        /// Name of the template that failed to load.
        name: String,
        /// Error that occurred while loading the template.
        error: Tl::Err,
    },
    /// Error while loading the font.
    Font {
        /// Name of the font that failed to load.
        name: String,
        /// Error that occurred while loading the font.
        error: Fl::Err,
    },
    /// Error while encoding the final image macro.
    Encode(io::Error),
}

impl<Tl, Fl> CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Create `CaptionError` for when a template failed to load.
    #[inline]
    pub fn template<N: ToString>(name: N, error: Tl::Err) -> Self {
        CaptionError::Template{ name: name.to_string(), error: error }
    }

    /// Create `CaptionError` for when a font failed to load.
    #[inline]
    pub fn font<N: ToString>(name: N, error: Fl::Err) -> Self {
        CaptionError::Font{ name: name.to_string(), error: error }
    }
}

impl<Tl, Fl> Error for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn description(&self) -> &str { "captioning error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            CaptionError::Template{ ref error, .. } => Some(error),
            CaptionError::Font{ ref error, .. } => Some(error),
            CaptionError::Encode(ref e) => Some(e),
        }
    }
}

impl<Tl, Fl> fmt::Debug for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template{ ref name, ref error } =>
                fmt.debug_struct("CaptionError::Template")
                    .field("name", name)
                    .field("error", &error.description())
                    .finish(),
            CaptionError::Font{ ref name, ref error } =>
                fmt.debug_struct("CaptionError::Font")
                    .field("name", name)
                    .field("error", &error.description())
                    .finish(),
            CaptionError::Encode(ref e) => write!(fmt, "CaptionError::Encode({:?})", e)
        }
    }
}

impl<Tl, Fl> fmt::Display for CaptionError<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CaptionError::Template{ ref name, ref error } =>
                write!(fmt, "cannot load template `{}`: {}", name, error.description()),
            CaptionError::Font{ ref name, ref error } =>
                write!(fmt, "cannot load font `{}`: {}", name, error.description()),
            CaptionError::Encode(ref e) => write!(fmt, "failed to encode the  final image: {}", e),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::CaptionError;

    #[test]
    fn thread_safe() {
        fn assert_sync<T: Sync>() {}
        fn assert_send<T: Send>() {}

        assert_sync::<CaptionError>();
        assert_send::<CaptionError>();
    }
}
