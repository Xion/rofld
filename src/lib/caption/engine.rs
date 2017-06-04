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
}

impl<Tl, Fl> Engine<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Render a given image macro,
    /// by captioning the template with the specified text(s).
    #[inline]
    pub fn caption(&self, image_macro: ImageMacro) -> Result<CaptionOutput, CaptionError> {
        CaptionTask::new(image_macro, self.inner.clone()).perform()
    }

    /// Return a reference to the internal template cache, if any.
    /// This can be used to examine cache statistics (hits & misses).
    #[inline]
    pub fn template_cache(&self) -> Option<&ThreadSafeCache<String, Tl::Item>> {
        if self.inner.template_loader.phony {
            None
        } else {
            Some(self.inner.template_loader.cache())
        }
    }

    /// Return a reference to the internal font cache, if any.
    /// This can be used to examine cache statistics (hits & misses).
    #[inline]
    pub fn font_cache(&self) -> Option<&ThreadSafeCache<String, Fl::Item>> {
        if self.inner.font_loader.phony {
            None
        } else {
            Some(self.inner.font_loader.cache())
        }
    }
}
