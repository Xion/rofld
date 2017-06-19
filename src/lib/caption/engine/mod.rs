//! Module which defines the captioning engine.

mod builder;
mod config;

pub use self::builder::Error as BuildError;
pub use self::config::{Config, Error as ConfigError};


use std::path::Path;
use std::sync::Arc;

use antidote::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use model::ImageMacro;
use resources::{CachingLoader, Font, FontLoader, Loader, Template, TemplateLoader};
use util::cache::ThreadSafeCache;
use super::error::CaptionError;
use super::output::CaptionOutput;
use super::task::CaptionTask;
pub use self::builder::Builder;


/// Image captioning engine.
///
/// The engine is thread-safe (`Sync`) since normally you'd want the captioning
/// to be performed in a background thread.
///
/// *Note*: `Engine` implements `Clone`
/// by merely cloning a shared reference to the underlying object.
#[derive(Clone, Debug)]
pub struct Engine<Tl = TemplateLoader, Fl = FontLoader>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    inner: Arc<Inner<Tl, Fl>>,
}

/// Shared state of the engine that caption tasks have access to.
#[derive(Debug)]
pub(super) struct Inner<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    pub(super) config: RwLock<Config>,
    pub template_loader: CachingLoader<Tl>,
    pub font_loader: CachingLoader<Fl>,
}

impl<Tl, Fl> Inner<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    #[inline]
    pub fn new(config: Config,
               template_loader: CachingLoader<Tl>,
               font_loader: CachingLoader<Fl>) -> Self {
        let config = RwLock::new(config);
        Inner{config, template_loader, font_loader}
    }
}

impl<Tl, Fl> From<Inner<Tl, Fl>> for Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn from(inner: Inner<Tl, Fl>) -> Self {
        Engine{inner: Arc::new(inner)}
    }
}

// Constructors.
impl Engine<TemplateLoader, FontLoader> {
    /// Create an Engine which loads templates & fonts from given directory paths.
    ///
    /// When loaded, both resources will be cached in memory (LRU cache).
    ///
    /// For other ways of creating `Engine`, see the `EngineBuilder`.
    #[inline]
    pub fn new<Dt, Df>(template_directory: Dt, font_directory: Df) -> Self
        where Dt: AsRef<Path>, Df: AsRef<Path>
    {
        Builder::new()
            .template_directory(template_directory)
            .font_directory(font_directory)
            .build().unwrap()
    }

    // TODO: consider deprecating all the other constructors now that we have a builder
}
impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Create an Engine that uses given loaders for templates & font.
    ///
    /// When loaded, both resources will be cached in memory (LRU cache).
    #[inline]
    pub fn with_loaders(template_loader: Tl, font_loader: Fl) -> Self {
        Builder::new()
            .template_loader(template_loader)
            .font_loader(font_loader)
            .build().unwrap()
    }

    /// Create an Engine that uses given template & font loaders directly.
    ///
    /// Any caching scheme, if necessary, should be implemented by loaders themselves.
    #[inline]
    pub fn with_raw_loaders(template_loader: Tl, font_loader: Fl) -> Self {
        Builder::new()
            .raw_template_loader(template_loader)
            .raw_font_loader(font_loader)
            .build().unwrap()    }
}


// Image macro captioning.
impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Render a given image macro by captioning the template with the specified text(s).
    ///
    /// Note that captioning is a CPU-intensive process and can be relatively lengthy,
    /// especially if the template is an animated GIF.
    /// It is recommended to execute it in a separate thread.
    #[inline]
    pub fn caption(&self, image_macro: ImageMacro) -> Result<CaptionOutput, CaptionError<Tl, Fl>> {
        CaptionTask::new(image_macro, self.inner.clone()).perform()
    }
}

// Managing resources.
impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Preemptively load a template into engine's cache.
    pub fn preload_template(&self, name: &str) -> Result<(), Tl::Err> {
        if !self.inner.template_loader.phony {
           self.inner.template_loader.load(name)?;
        }
        Ok(())
    }

    /// Preemptively load a font into engine's cache.
    pub fn preload_font(&self, name: &str) -> Result<(), Fl::Err> {
        if !self.inner.font_loader.phony {
            self.inner.font_loader.load(name)?;
        }
        Ok(())
    }

    /// Return a reference to the internal template cache, if any.
    /// This can be used to examine cache statistics (hits & misses).
    pub fn template_cache(&self) -> Option<&ThreadSafeCache<String, Tl::Item>> {
        if self.inner.template_loader.phony {
            None
        } else {
            Some(self.inner.template_loader.cache())
        }
    }

    /// Return a reference to the internal font cache, if any.
    /// This can be used to examine cache statistics (hits & misses).
    pub fn font_cache(&self) -> Option<&ThreadSafeCache<String, Fl::Item>> {
        if self.inner.font_loader.phony {
            None
        } else {
            Some(self.inner.font_loader.cache())
        }
    }
}

// Configuration.
impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Read the `Engine`'s configuration.
    #[inline]
    pub fn config(&self) -> RwLockReadGuard<Config> {
        self.inner.config.read()
    }

    /// Modify the `Engine`'s configuration.
    ///
    /// Changes will affect both pending and future captioning tasks.
    #[inline]
    pub fn config_mut(&self) -> RwLockWriteGuard<Config> {
        self.inner.config.write()
    }
}


#[cfg(test)]
mod tests {
    use super::Engine;

    #[test]
    fn thread_safe() {
        fn assert_sync<T: Sync>() {}
        fn assert_send<T: Send>() {}

        assert_sync::<Engine>();
        assert_send::<Engine>();
    }
}
