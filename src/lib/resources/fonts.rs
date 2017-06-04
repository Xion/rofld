//! Module for loading fonts used in image macros.

use std::error::Error;
use std::fmt;
use std::path::Path;

use rusttype::{self, FontCollection};

use super::Loader;
use super::filesystem::{BytesLoader, FileLoader};


pub const FILE_EXTENSION: &'static str = "ttf";


/// Font that can be used to caption image macros.
macro_attr! {
    #[derive(NewtypeDeref!, NewtypeFrom!)]
    pub struct Font(rusttype::Font<'static>);
    // TODO: add font name for better Debug
}
impl fmt::Debug for Font {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Font(...)")
    }
}


#[derive(Debug)]
pub struct FontLoader {
    inner: BytesLoader<'static>,
}

impl FontLoader {
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        FontLoader{
            inner: BytesLoader::new(
                FileLoader::for_extension(directory, FILE_EXTENSION))
        }
    }
}

impl Loader for FontLoader {
    type Item = Font;
    type Err = Box<Error>; // TODO: implement an error type.

    fn load<'n>(&self, name: &'n str) -> Result<Font, Self::Err> {
        let bytes = self.inner.load(name)
            .map_err(|_| "Can't load font")?;

        let fonts: Vec<_> = FontCollection::from_bytes(bytes).into_fonts().collect();
        match fonts.len() {
            0 => {
                error!("No fonts in a file for `{}` font resource", name);
                Err("0 fonts".into())
            }
            1 => {
                debug!("Font `{}` loaded successfully", name);
                Ok(fonts.into_iter().next().unwrap().into())
            }
            _ => {
                error!("Font file for `{}` resource contains {} fonts, expected one",
                    name, fonts.len());
                Err(">1 font".into())
            }
        }
    }
}
