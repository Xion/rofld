//! Module which defines the captioning engine.

use std::path::Path;
use std::sync::Arc;

use model::ImageMacro;
use resources::{CachingLoader, Font, FontLoader, Loader, Template, TemplateLoader};
use util::cache::ThreadSafeCache;
use super::error::CaptionError;
use super::output::CaptionOutput;
use super::task::CaptionTask;


const DEFAULT_TEMPLATE_CAPACITY: usize = 128;
const DEFAULT_FONT_CAPACITY: usize = 16;


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
    pub template_loader: CachingLoader<Tl>,
    pub font_loader: CachingLoader<Fl>,
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
    /// When loaded, both resources will be cached in memory (LRU cache).
    #[inline]
    pub fn new<Dt, Df>(template_directory: Dt, font_directory: Df) -> Self
        where Dt: AsRef<Path>, Df: AsRef<Path>
    {
        Engine::with_loaders(
            TemplateLoader::new(template_directory),
            FontLoader::new(font_directory),
        )
    }
}
impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Create an Engine that uses given loaders for templates & font.
    /// When loaded, both resources will be cached in memory (LRU cache).
    #[inline]
    pub fn with_loaders(template_loader: Tl, font_loader: Fl) -> Self {
        Engine::from(Inner{
            template_loader: CachingLoader::new(template_loader, DEFAULT_TEMPLATE_CAPACITY),
            font_loader: CachingLoader::new(font_loader, DEFAULT_FONT_CAPACITY),
        })
    }

    /// Create an Engine that uses given template & font loaders directly.
    /// Any caching scheme, if necessary, should be implemented by loaders themselves.
    #[inline]
    pub fn with_raw_loaders(template_loader: Tl, font_loader: Fl) -> Self {
        // Use the phony version of CachingLoader which doesn't actually cache anything,
        // but provides the same interface yielding Arc<T>.
        Engine::from(Inner{
            template_loader: CachingLoader::phony(template_loader),
            font_loader: CachingLoader::phony(font_loader),
        })
    }

    // TODO: add a Builder that also allows to:
    // * set the capacity of standard template/font caches
    // * adjust GIF & JPEG quality parameters
    // * adjust memory limit for GIF decoding
    // (this will probably warrant a separate Config struct)
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
