//! Module handling the resources used for captioning.

mod filesystem;
mod fonts;
mod templates;


pub use self::fonts::{Font, FontLoader, FILE_EXTENSION as FONT_FILE_EXTENSION};
pub use self::templates::{DEFAULT_IMAGE_FORMAT, IMAGE_FORMAT_EXTENSIONS,
                          Template, TemplateLoader, TemplateError};


use std::fmt;
use std::sync::Arc;

use util::cache::ThreadSafeCache;


/// Loader of resources from some external source.
pub trait Loader {
    /// Type of resources that this loader can load.
    type Item;
    /// Error that may occur while loading the resource.
    type Err;
    // TODO: add an Error bound if this is ever resolved:
    // https://github.com/rust-lang/rust/pull/30796#issuecomment-171085915
    // or the TODO in FontLoader is fixed

    /// Load a resource of given name.
    fn load<'n>(&self, name: &'n str) -> Result<Self::Item, Self::Err>;
}

/// Type of a loader that doles out shared references to the resources.
pub type SharingLoader<T, E> = Loader<Item=Arc<T>, Err=E>;


/// A loader that keeps a cache of resources previously loaded.
pub struct CachingLoader<L: Loader> {
    inner: L,
    cache: ThreadSafeCache<String, L::Item>,
    pub(crate) phony: bool,
}

impl<L: Loader> CachingLoader<L> {
    #[inline]
    pub fn new(inner: L, capacity: usize) -> Self {
        CachingLoader{
            inner: inner,
            cache: ThreadSafeCache::new(capacity),
            phony: false,
        }
    }

    /// Create a phony version of CachingLoader that doesn't actually cache anything.
    ///
    /// This is used to transparently wrap a Loader<Item=T> into Loader<Item=Arc<T>>,
    /// which is necessary because Rust cannot really abstract between the two.
    #[inline]
    pub(crate) fn phony(inner: L) -> Self {
        CachingLoader{
            inner: inner,
            cache: ThreadSafeCache::new(1),
            phony: true,
        }
    }
}

impl<L: Loader> CachingLoader<L> {
    #[inline]
    pub fn cache(&self) -> &ThreadSafeCache<String, L::Item> {
        &self.cache
    }
}

impl<L: Loader> Loader for CachingLoader<L> {
    type Item = Arc<L::Item>;
    type Err = L::Err;

    /// Load the object from cache or fall back on the original Loader.
    /// Cache the objects loaded this way.
    fn load<'n>(&self, name: &'n str) -> Result<Self::Item, Self::Err> {
        if self.phony {
            let obj = self.inner.load(name)?;
            Ok(Arc::new(obj))
        } else {
            if let Some(obj) = self.cache.get(name) {
                return Ok(obj);
            }
            let obj = self.inner.load(name)?;
            let cached_obj = self.cache.put(name.to_owned(), obj);
            Ok(cached_obj)
        }
    }
}

impl<L: Loader> fmt::Debug for CachingLoader<L> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("CachingLoader")
            .field("inner", &"...")
            .field("cache", &self.cache)
            .field("phony", &self.phony)
            .finish()
    }
}
