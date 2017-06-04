//! Module defining and implementing resource loaders.

use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::iter;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glob;

use super::{Loader, ThreadSafeCache};


/// Loader for file paths from given directory.
///
/// The resources here are just file *paths* (std::path::PathBuf),
/// and no substantial "loading" is performing (only path resolution).
///
/// This isn't particularly useful on its own, but can be wrapped around
/// to make more interesting loaders.
pub struct PathLoader<'pl> {
    directory: PathBuf,
    predicate: Arc<Fn(&Path) -> bool + Send + Sync + 'pl>,
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
        where D: AsRef<Path>, P: Fn(&Path) -> bool + Send + Sync + 'pl
    {
        PathLoader{
            directory: directory.as_ref().to_owned(),
            predicate: Arc::new(predicate),
        }
    }
}

impl<'pl> Loader for PathLoader<'pl> {
    type Item = PathBuf;
    type Err = io::Error;

    /// "Load" a path "resource" from the loader's directory.
    fn load<'n>(&self, name: &'n str) -> Result<Self::Item, Self::Err> {
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

impl<'pl> fmt::Debug for PathLoader<'pl> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("PathLoader")
            .field("directory", &self.directory)
            .finish()
    }
}


/// Loader for files in given directory.
///
/// The resources it doles out are just file handles (std::fs::File).
/// Wrappers around this loaded can then implement their own decoding.
#[derive(Debug)]
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
        where D: AsRef<Path>, P: Fn(&Path) -> bool + Send + Sync + 'pl
    {
        FileLoader{inner: PathLoader::with_predicate(directory, predicate)}
    }

    // TODO: add filtering based on file metadata too
}

impl<'pl> Loader for FileLoader<'pl> {
    type Item = File;
    type Err = io::Error;

    fn load<'n>(&self, name: &'n str) -> Result<Self::Item, Self::Err> {
        let path = self.inner.load(name)?;
        fs::OpenOptions::new().read(true).open(path)
    }
}


/// Wrapper around FileLoader that loads the entire content of the files.
#[derive(Debug)]
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

impl<'fl> Loader for BytesLoader<'fl> {
    type Item = Vec<u8>;
    type Err = io::Error;

    /// Load a file resource as its byte content.
    fn load<'n>(&self, name: &'n str) -> Result<Self::Item, Self::Err> {
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
