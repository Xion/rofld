//! MOdule implementing cache for data needed to create image macros.
//! Currently, this includes image templates and fonts.

use std::sync::{Arc, Mutex};

use image::DynamicImage;
use lru_cache::LruCache;
use rusttype::Font;

use super::{fonts, templates};


// TODO: make those settable from the command line
const TEMPLATE_CACHE_SIZE: usize = 128;
const FONT_CACHE_SIZE: usize = 16;


/// Cache for data used in rendering of image macros.
/// The cache is designed to be thread-safe.
pub struct Cache {
    templates: Mutex<LruCache<String, Arc<DynamicImage>>>,
    fonts: Mutex<LruCache<String, Arc<Font<'static>>>>,
}
unsafe impl Sync for Cache {}

impl Cache {
    #[inline]
    pub fn new() -> Self {
        Cache{
            templates: Mutex::new(LruCache::new(TEMPLATE_CACHE_SIZE)),
            fonts: Mutex::new(LruCache::new(FONT_CACHE_SIZE)),
        }
    }
}

impl Cache {
    /// Get the image for a template of given name.
    /// If it doesn't exist in the cache, it will be loaded & cached.
    pub fn get_template(&self, name: &str) -> Option<Arc<DynamicImage>> {
        // Try to hit the cache quickly first.
        {
            let mut tmpl_cache = self.templates.lock()
                .expect("Cache::templates lock poisoned");
            if let Some(img) = tmpl_cache.get_mut(name) {
                trace!("Cache hit for template `{}`", name);
                return Some(img.clone());
            }
        }
        debug!("Cache miss for template `{}`", name);

        // Load the image template outside of the critical section.
        if let Some(img) = templates::load(name) {

            let img = Arc::new(img);
            self.templates.lock().expect("Cache::templates lock poisoned")
                .insert(name.to_owned(), img.clone());
            trace!("Template `{}` cached", name);
            return Some(img);
        }

        None
    }

    /// Get the font with given name.
    /// If it doesn't exist in the cache, it will be loaded & cached.
    pub fn get_font(&self, name: &str) -> Option<Arc<Font<'static>>> {
        // Try to hit the cache quickly first.
        {
            let mut font_cache = self.fonts.lock()
                .expect("Cache::fonts lock poisoned");
            if let Some(font) = font_cache.get_mut(name) {
                trace!("Cache hit for font `{}`", name);
                return Some(font.clone());
            }
        }
        debug!("Cache miss for font `{}`", name);

        // Load the font outside of a critical section.
        if let Some(font) = fonts::load(name) {
            let font = Arc::new(font);
            self.fonts.lock().expect("Cache::fonts lock poisoned")
                .insert(name.to_owned(), font.clone());
            trace!("Font `{}` cached", name);
            return Some(font);
        }

        None
    }
}
