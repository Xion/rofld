//! Module implementing the builder for `Engine`.

use std::fmt::Display;
use std::mem;
use std::path::{Path, PathBuf};

use either::*;

use ext::rust::OptionMutExt;
use resources::{CachingLoader, Font, FontLoader, Loader, Template, TemplateLoader};
use super::config::Config;
use super::Engine;


const DEFAULT_TEMPLATE_CAPACITY: usize = 128;
const DEFAULT_FONT_CAPACITY: usize = 16;


/// Builder for `Engine`.
#[derive(Debug)]
#[must_use = "unused builder which must be used"]
pub struct Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    errors: Vec<Error>,

    template_loader_builder: Option<LoaderBuilder<Tl>>,
    font_loader_builder: Option<LoaderBuilder<Fl>>,

    jpeg_quality: Option<u8>,
    // TODO: add gif_quality
}


/// Temporary configuration for a template or font loader.
/// Used by `Builder`.
#[derive(Debug)]
enum LoaderBuilder<L: Loader> {
    Cached {
        inner: Option<Either<L, PathBuf>>,
        cache_size: usize,
    },
    Raw { inner: Option<L> },
}

impl<L: Loader> LoaderBuilder<L> {
    #[inline]
    pub fn cached(size: usize) -> Self {
        LoaderBuilder::Cached { inner: None, cache_size: size }
    }

    #[inline]
    pub fn raw() -> Self {
        LoaderBuilder::Raw { inner: None }
    }
}

// WTB lenses from Haskell :-(
impl<L: Loader> LoaderBuilder<L> {
    /// Set the directory that the `Cached` loader would use.
    /// Returns `false` if `LoaderBuilder` is already configured incompatibly.
    pub fn set_cached_loader_directory<P: AsRef<Path>>(&mut self, directory: P) -> bool {
        match self {
            &mut LoaderBuilder::Cached{ref mut inner, ..} => {
                if inner.as_ref().map(|i| i.is_right()).unwrap_or(true) {
                    *inner = Some(Right(directory.as_ref().to_owned()));
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Set the loader that the `Cached` loader would wrap.
    /// Returns `false` if `LoaderBuilder` is already configured incompatibly.
    pub fn set_cached_loader_object(&mut self, loader: L) -> bool {
        match self {
            &mut LoaderBuilder::Cached{ref mut inner, ..} => {
                if inner.as_ref().map(|i| i.is_left()).unwrap_or(true) {
                    *inner = Some(Left(loader));
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Set the cache size of `Cached` loader.
    /// Returns `false` if `LoaderBuilder` is already configured incompatibly.
    pub fn set_cached_size(&mut self, size: usize) -> bool {
        match self {
            &mut LoaderBuilder::Cached{ref mut cache_size, ..} => {
                *cache_size = size;
                return true;
            }
            _ => {}
        }
        false
    }

    /// Set the loader that the `Raw` loader would wrap.
    /// Returns `false` if `LoaderBuilder` is already configured incompatibly.
    pub fn set_raw_loader_object(&mut self, loader: L) -> bool {
        match self {
            &mut LoaderBuilder::Raw{ref mut inner} => {
                *inner = Some(loader);
                return true;
            }
            _ => {}
        }
        false
    }
}


impl<Tl, Fl> Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Create a new `Builder`.
    #[inline]
    pub fn new() -> Self {
        Builder::default()
    }
}
impl<Tl, Fl> Default for Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    fn default() -> Self {
        Builder{
            errors: vec![],
            template_loader_builder: None,
            font_loader_builder: None,
            jpeg_quality: None,
        }
    }
}

// Setters.
impl<Fl> Builder<TemplateLoader, Fl>
    where Fl: Loader<Item=Font>
{
    /// Set the directory where the templates will be loaded from.
    #[inline]
    pub fn template_directory<P: AsRef<Path>>(mut self, directory: P) -> Self {
        self.template_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_TEMPLATE_CAPACITY));
        let ok = self.template_loader_builder.as_mut().unwrap()
            .set_cached_loader_directory(directory);
        if ok { self } else { self.err(Error::loader_setup_conflict("template")) }
    }
}
impl<Tl> Builder<Tl, FontLoader>
    where Tl: Loader<Item=Template>
{
    /// Set the directory where the fonts will be loaded from.
    #[inline]
    pub fn font_directory<P: AsRef<Path>>(mut self, directory: P) -> Self {
        self.font_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_FONT_CAPACITY));
        let ok = self.font_loader_builder.as_mut().unwrap()
            .set_cached_loader_directory(directory);
        if ok { self } else { self.err(Error::loader_setup_conflict("font")) }
    }
}
impl<Tl, Fl> Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Set a custom loader for templates.
    ///
    /// Templates loaded by it will still be cached in an LRU cache.
    /// See `raw_template_loader` if you want to provide your own caching.
    #[inline]
    pub fn template_loader(mut self, loader: Tl) -> Self {
        self.template_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_TEMPLATE_CAPACITY));
        let ok = self.template_loader_builder.as_mut().unwrap()
            .set_cached_loader_object(loader);
        if ok { self } else { self.err(Error::loader_setup_conflict("template")) }
    }

    /// Change the size of the template cache.
    #[inline]
    pub fn template_cache_size(mut self, size: usize) -> Self {
        self.template_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_TEMPLATE_CAPACITY));
        let ok = self.template_loader_builder.as_mut().unwrap()
            .set_cached_size(size);
        if ok { self } else { self.err(Error::loader_setup_conflict("template")) }
    }

    /// Set a custom loader for fonts.
    ///
    /// Fonts loaded by it will still be cached in an LRU cache.
    /// See `raw_font_loader` if you want to provide your own caching.
    #[inline]
    pub fn font_loader(mut self, loader: Fl) -> Self {
        self.font_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_FONT_CAPACITY));
        let ok = self.font_loader_builder.as_mut().unwrap()
            .set_cached_loader_object(loader);
        if ok { self } else { self.err(Error::loader_setup_conflict("font")) }
    }

    /// Change the size of the font cache.
    #[inline]
    pub fn font_cache_size(mut self, size: usize) -> Self {
        self.font_loader_builder
            .set_default_with(|| LoaderBuilder::cached(DEFAULT_FONT_CAPACITY));
        let ok = self.font_loader_builder.as_mut().unwrap()
            .set_cached_size(size);
        if ok { self } else { self.err(Error::loader_setup_conflict("font")) }
    }

    /// Set a custom "raw" loader for templates.
    ///
    /// Templates loaded this way will not be cached (unless the loader itself
    /// implements some kind of caching).
    #[inline]
    pub fn raw_template_loader(mut self, loader: Tl) -> Self {
        self.template_loader_builder.set_default_with(LoaderBuilder::raw);
        let ok = self.template_loader_builder.as_mut().unwrap()
            .set_raw_loader_object(loader);
        if ok { self } else { self.err(Error::loader_setup_conflict("template")) }
    }

    /// Set a custom "raw" loader for fonts.
    ///
    /// Fonts loaded this way will not be cached (unless the loader itself
    /// implements some kind of caching).
    #[inline]
    pub fn raw_font_loader(mut self, loader: Fl) -> Self {
        self.font_loader_builder.set_default_with(LoaderBuilder::raw);
        let ok = self.font_loader_builder.as_mut().unwrap()
            .set_raw_loader_object(loader);
        if ok { self } else { self.err(Error::loader_setup_conflict("font")) }
    }
}
impl<Tl, Fl> Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Set the quality percentage of JPEG images generated by the `Engine`.
    #[inline]
    pub fn jpeg_quality(mut self, quality: u8) -> Self {
        self.jpeg_quality = Some(quality); self
    }
}

// Validation & building.
impl<Tl, Fl> Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    /// Build the `Engine`.
    pub fn build(self) -> Result<Engine<Tl, Fl>, Error> {
        self.check_errors()?;

        let config = self.build_config();
        let template_loader = self.template_loader_builder
            .ok_or_else(|| Error::no_loader_for("template"))?
            .build(|d| TemplateLoader::new(d))?;
        let font_loader = self.font_loader_builder
            .ok_or_else(|| Error::no_loader_for("font"))?
            .build(|d| FontLoader::new(d))?;
        Ok(Engine::from(super::Inner{config, template_loader, font_loader}))
    }
}

// Utilities for validation & building.
impl<L: Loader> LoaderBuilder<L> {
    /// Build the `Loader`.
    ///
    /// The closure passed is for the case when this is a standard loader we're building,
    /// like the `TemplateLoader`.
    pub fn build<F, Sl>(self, standard_ctor: F) -> Result<CachingLoader<L>, Error>
        where F: FnOnce(PathBuf) -> Sl, Sl: Loader
    {
        match self {
            LoaderBuilder::Cached{ inner, cache_size } => match inner {
                None => Err(Error::no_loader()),
                Some(Left(loader)) => Ok(CachingLoader::new(loader, cache_size)),
                Some(Right(directory)) => {
                    //
                    // It is sadly impossible to prove statically, but if we got here,
                    // it means that `L` is actually equal to `Sl`.
                    //
                    // "Proof":
                    //   This is because the only way to set the `Right` variant here is through
                    //   `Builder::(template|font)_directory`, and these are only implemented
                    //   for `Builder<TemplateLoader|FontLoader>`, which in turn means `L`
                    //   has been specialized with one of those two in the first place.
                    //
                    //   And finally, this method is only called by `Builder::build`
                    //   which makes the correct decision between those two standard loaders.
                    //
                    // Since we need to return a result with `L`, we take advantage of this
                    // equality to subvert the type system. I'm sorry.
                    //
                    let standard_loader = standard_ctor(directory);
                    let loader = unsafe { mem::transmute_copy::<Sl, L>(&standard_loader) };
                    mem::forget(standard_loader);

                    Ok(CachingLoader::new(loader, cache_size))
                }
            },
            LoaderBuilder::Raw{ inner } => {
                // Use the phony version of CachingLoader which doesn't actually cache anything,
                // but provides the same interface yielding Arc<L::Item>.
                Ok(CachingLoader::phony(inner
                    .expect("raw loader in LoaderBuilder::build_custom")))
            }
        }
    }
}
impl<Tl, Fl> Builder<Tl, Fl>
    where Tl: Loader<Item=Template>, Fl: Loader<Item=Font>
{
    #[doc(hidden)]
    fn build_config(&self) -> Config {
        let mut config = Config::default();
        if let Some(quality) = self.jpeg_quality {
            // TODO: validate this, should be <=100
            config.jpeg_quality = quality;
        }
        config
    }

    #[doc(hidden)]
    fn check_errors(&self) -> Result<(), Error> {
        if !self.errors.is_empty() {
            // TODO: consider making a Error::Multiple variant to return them all
            return Err(self.errors.iter().next().unwrap().clone());
        }
        Ok(())
    }

    #[doc(hidden)]
    fn err(mut self, error: Error) -> Self {
        self.errors.push(error); self
    }
}


/// Error that resulted from misconfiguration of the `Engine` via its `Builder`.
#[derive(Clone, Debug, Error)]
pub enum Error {
    // TODO: use an enum for Template/Font resource rather than a String messages
    // (along with with custom Display impl)

    /// No loader set up.
    #[error(msg_embedded, non_std, no_from)]
    NoLoader(String),

    /// Template or font loader configuration setup error.
    #[error(msg_embedded, non_std, no_from)]
    LoaderBuilderConflict(String),
}

impl Error {
    #[inline]
    pub(super) fn no_loader() -> Self {
        Error::NoLoader("missing loader configuration".into())
    }

    #[inline]
    pub(super) fn no_loader_for<R: Display>(resource: R) -> Self {
        Error::NoLoader(format!("no {} loader configured", resource))
    }

    #[inline]
    pub(super) fn loader_setup_conflict<R: Display>(resource: R) -> Self {
        Error::LoaderBuilderConflict(format!(
            "invalid combination of configuration parameters for setting up {} loader", resource))
    }
}
