//! Module defining and implementing resource loaders.

use std::error::Error;
use std::fs::{self, File};
use std::iter;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use image::ImageError;
use glob;
use rusttype::{Font, FontCollection};

use super::ThreadSafeCache;
use super::templates::{Template, TemplateError};


/// Loader of resources T from some external source.
pub trait Loader<T> {
    /// Error that may occur while loading the resource.
    type Err;
    // TODO: add an Error bound if this is ever resolved:
    // https://github.com/rust-lang/rust/pull/30796#issuecomment-171085915
    // or the TODO in FontLoader is fixed

    /// Load a resource of given name.
    fn load<'n>(&self, name: &'n str) -> Result<T, Self::Err>;
}

/// Type of a loader that doles out shared references to the resources.
pub type SharingLoader<T, E> = Loader<Arc<T>, Err=E>;


/// A loader that keeps a cache of resources previously loaded.
pub struct CachingLoader<T, E, L>
    where E: Error, L: Loader<T, Err=E>
{
    inner: L,
    cache: ThreadSafeCache<String, T>,
}

impl<T, E, L> CachingLoader<T, E, L>
    where E: Error, L: Loader<T, Err=E>
{
    #[inline]
    pub fn new(inner: L, capacity: usize) -> Self {
        CachingLoader{
            inner: inner,
            cache: ThreadSafeCache::new(capacity),
        }
    }
}

impl<T, E, L> Loader<Arc<T>> for CachingLoader<T, E, L>
    where E: Error, L: Loader<T, Err=E>
{
    type Err = E;

    /// Load the object from cache or fall back on the original Loader.
    /// Cache the objects loaded this way.
    fn load<'n>(&self, name: &'n str) -> Result<Arc<T>, Self::Err> {
        if let Some(obj) = self.cache.get(name) {
            return Ok(obj);
        }
        let obj = self.inner.load(name)?;
        let cached_obj = self.cache.put(name.to_owned(), obj);
        Ok(cached_obj)
    }
}


/// Loader for file paths from given directory.
///
/// The resources here are just file *paths* (std::path::PathBuf),
/// and no substantial "loading" is performing (only path resolution).
///
/// This isn't particularly useful on its own, but can be wrapped around
/// to make more interesting loaders.
pub struct PathLoader<'pl> {
    directory: PathBuf,
    predicate: Box<Fn(&Path) -> bool + 'pl>,
}

impl<'pl> PathLoader<'pl> {
    #[inline]
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        Self::with_predicate(directory, |_| true)
    }

    #[inline]
    pub fn for_extension<D: AsRef<Path>, S>(directory: D, extension: S) -> Self
        where S: ToString
    {
        Self::for_extensions(directory, iter::once(extension))
    }

    /// Create a loader which only gives out paths to files
    /// that have one of the extensions given.
    pub fn for_extensions<D: AsRef<Path>, I, S>(directory: D, extensions: I) -> Self
        where I: IntoIterator<Item=S>, S: ToString
    {
        Self::with_predicate(directory, {
            let extensions: Vec<_> = extensions.into_iter()
                .map(|e| e.to_string()).map(|e| e.trim().to_lowercase())
                .collect();
            move |path| {
                let ext = path.extension().and_then(|e| e.to_str())
                    .map(|s| s.trim().to_lowercase());
                extensions.iter().any(|e| Some(e) == ext.as_ref())
            }
        })
    }

    pub fn with_predicate<D, P>(directory: D, predicate: P) -> Self
        where D: AsRef<Path>, P: Fn(&Path) -> bool + 'pl
    {
        PathLoader{
            directory: directory.as_ref().to_owned(),
            predicate: Box::new(predicate),
        }
    }
}

impl<'pl> Loader<PathBuf> for PathLoader<'pl> {
    type Err = io::Error;

    /// "Load" a path "resource" from the loader's directory.
    fn load<'n>(&self, name: &'n str) -> Result<PathBuf, Self::Err> {
        let file_part = format!("{}.*", name);
        let pattern = format!("{}", self.directory.join(file_part).display());
        trace!("Globbing with {}", pattern);

        let glob_iter = match glob::glob(&pattern) {
            Ok(it) => it,
            Err(e) => {
                error!("Failed to glob over files with {}: {}", pattern, e);
                return Err(io::Error::new(io::ErrorKind::Other, e));
            },
        };
        let matches: Vec<_> = glob_iter
            .filter_map(Result::ok)  // TODO: report those errors
            .filter(|f| (self.predicate)(f))
            .collect();

        match matches.len() {
            0 => Err(io::Error::new(io::ErrorKind::NotFound,
                format!("resource `{}` not found in {}", name, self.directory.display()))),
            1 => Ok(matches.into_iter().next().unwrap()),
            c => Err(io::Error::new(io::ErrorKind::InvalidInput,
                format!("ambiguous resource name `{}` matching {} files in {}",
                    name, c, self.directory.display()))),
        }
    }
}


/// Loader for files in given directory.
///
/// The resources it doles out are just file handles (std::fs::File).
/// Wrappers around this loaded can then implement their own decoding.
pub struct FileLoader<'pl> {
    inner: PathLoader<'pl>,
}

// Constructors that for convenience are delegating to the PathLoader ones.
impl<'pl> FileLoader<'pl> {
    #[inline]
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        FileLoader{inner: PathLoader::new(directory)}
    }

    #[inline]
    pub fn for_extension<D: AsRef<Path>, S>(directory: D, extension: S) -> Self
        where S: ToString
    {
        FileLoader{inner: PathLoader::for_extension(directory, extension)}
    }

    /// Create a loader which only loads files
    /// that have one of the extensions given.
    #[inline]
    pub fn for_extensions<D: AsRef<Path>, I, S>(directory: D, extensions: I) -> Self
        where I: IntoIterator<Item=S>, S: ToString
    {
        FileLoader{inner: PathLoader::for_extensions(directory, extensions)}
    }

    #[inline]
    pub fn with_path_predicate<D, P>(directory: D, predicate: P) -> Self
        where D: AsRef<Path>, P: Fn(&Path) -> bool + 'pl
    {
        FileLoader{inner: PathLoader::with_predicate(directory, predicate)}
    }

    // TODO: add filtering based on file metadata too
}

impl<'pl> Loader<File> for FileLoader<'pl> {
    type Err = io::Error;

    fn load<'n>(&self, name: &'n str) -> Result<File, Self::Err> {
        let path = self.inner.load(name)?;
        fs::OpenOptions::new().read(true).open(path)
    }
}


/// Wrapper around FileLoader that loads the entire content of the files.
pub struct BytesLoader<'fl> {
    inner: FileLoader<'fl>,
}

impl<'fl> BytesLoader<'fl> {
    #[inline]
    pub fn new(inner: FileLoader<'fl>) -> Self {
        BytesLoader{inner}
    }
}
impl<'fl> From<FileLoader<'fl>> for BytesLoader<'fl> {
    fn from(input: FileLoader<'fl>) -> Self {
        Self::new(input)
    }
}

impl<'fl> Loader<Vec<u8>> for BytesLoader<'fl> {
    type Err = io::Error;

    /// Load a file resource as its byte content.
    fn load<'n>(&self, name: &'n str) -> Result<Vec<u8>, Self::Err> {
        let file = self.inner.load(name)?;

        let mut bytes = match file.metadata() {
            Ok(stat) => Vec::with_capacity(stat.len() as usize),
            Err(e) => {
                warn!("Failed to stat file of resource `{}` to obtain its size: {}",
                    name, e);
                Vec::new()
            },
        };

        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}


pub struct TemplateLoader {
    inner: PathLoader<'static>,
}

impl TemplateLoader {
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        let extensions = &["gif", "jpg", "jpeg", "png"];
        TemplateLoader{
            inner: PathLoader::for_extensions(directory, extensions),
        }
    }
}

impl Loader<Template> for TemplateLoader {
    type Err = TemplateError;

    fn load<'n>(&self, name: &'n str) -> Result<Template, Self::Err> {
        // TODO: add FileError variant to TemplateError
        let path = self.inner.load(name).map_err(ImageError::IoError)?;
        // TODO: move the loading code here from try_from()
        use conv::TryFrom;
        Template::try_from(path)
    }
}


pub struct FontLoader {
    inner: BytesLoader<'static>,
}

impl FontLoader {
    pub fn new<D: AsRef<Path>>(directory: D) -> Self {
        FontLoader{
            inner: BytesLoader::new(
                FileLoader::for_extension(directory, "ttf"))
        }
    }
}

impl Loader<Font<'static>> for FontLoader {
    type Err = Box<Error>; // TODO: implement an error type.

    fn load<'n>(&self, name: &'n str) -> Result<Font<'static>, Self::Err> {
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
                Ok(fonts.into_iter().next().unwrap())
            }
            _ => {
                error!("Font file for `{}` resource contains {} fonts, expected one",
                    name, fonts.len());
                Err(">1 font".into())
            }
        }
    }
}
