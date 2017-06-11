//! Module for loading fonts used in image macros.

use std::error::Error;
use std::fmt;
use std::io;
use std::path::Path;

use rusttype::{self, FontCollection};

use super::Loader;
use super::filesystem::{BytesLoader, FileLoader};


/// File extension of font files.
pub const FILE_EXTENSION: &'static str = "ttf";


macro_attr! {
    /// Font that can be used to caption image macros.
    #[derive(NewtypeDeref!, NewtypeFrom!)]
    pub struct Font(rusttype::Font<'static>);
    // TODO: add font name for better Debug
}
impl fmt::Debug for Font {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Font(...)")
    }
}


/// Error that may occur during font load.
#[derive(Debug)]
pub enum FontError {
    /// Error while loading the font file.
    File(io::Error),
    /// Error for when the font file contains no fonts.
    NoFonts,
    /// Error for when the font file contains too many fonts.
    TooManyFonts(usize),
}

impl Error for FontError {
    fn description(&self) -> &str { "font loading error" }
    fn cause(&self) -> Option<&Error> {
        match *self {
            FontError::File(ref e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for FontError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FontError::File(ref e) => write!(fmt, "I/O error while loading font: {}", e),
            FontError::NoFonts => write!(fmt, "no fonts found in the file"),
            FontError::TooManyFonts(c) =>
                write!(fmt, "expected a single font in the file, found {}", c),
        }
    }
}


/// Loader for fonts stored in a directory.
///
/// Font names are translated directly into file names, loaded, and cached.
#[derive(Debug)]
pub struct FontLoader {
    inner: BytesLoader<'static>,
}

impl FontLoader {
    /// Create a new font loader.
    #[inline]
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        FontLoader{
            inner: BytesLoader::new(
                FileLoader::for_extension(directory, FILE_EXTENSION))
        }
    }
}

impl Loader for FontLoader {
    type Item = Font;
    type Err = FontError;

    fn load<'n>(&self, name: &'n str) -> Result<Font, Self::Err> {
        let bytes = self.inner.load(name).map_err(FontError::File)?;

        let fonts: Vec<_> = FontCollection::from_bytes(bytes).into_fonts().collect();
        match fonts.len() {
            0 => {
                error!("No fonts in a file for `{}` font resource", name);
                Err(FontError::NoFonts)
            }
            1 => {
                debug!("Font `{}` loaded successfully", name);
                Ok(fonts.into_iter().next().unwrap().into())
            }
            count => {
                error!("Font file for `{}` resource contains {} fonts, expected one",
                    name, count);
                Err(FontError::TooManyFonts(count))
            }
        }
    }
}
