//! MOdule implementing cache for data needed to create image macros.
//! Currently, this includes image templates and fonts.

use std::sync::{Arc, Mutex};

use lru_cache::LruCache;
use rusttype::Font;

use super::fonts;
use super::templates::{self, Template};


const DEFAULT_TEMPLATE_CAPACITY: usize = 128;
const DEFAULT_FONT_CAPACITY: usize = 16;


/// Cache for data used in rendering of image macros.
/// The cache is designed to be thread-safe.
pub struct Cache {
    templates: Mutex<LruCache<String, Arc<Template>>>,
    fonts: Mutex<LruCache<String, Arc<Font<'static>>>>,
}
unsafe impl Sync for Cache {}

impl Cache {
    #[inline]
    pub fn new() -> Self {
        Cache{
            templates: Mutex::new(LruCache::new(DEFAULT_TEMPLATE_CAPACITY)),
            fonts: Mutex::new(LruCache::new(DEFAULT_FONT_CAPACITY)),
        }
    }
}

impl Cache {
    #[inline]
    pub fn set_template_capacity(&self, capacity: usize) -> &Self {
        self.templates.lock()
            .expect("Cache::templates lock poisoned")
            .set_capacity(capacity);
        self
    }

    #[inline]
    pub fn set_font_capacity(&self, capacity: usize) -> &Self {
        self.fonts.lock()
            .expect("Cache::fonts lock poisoned")
            .set_capacity(capacity);
        self
    }
}

impl Cache {
    /// Get the image for a template of given name.
    /// If it doesn't exist in the cache, it will be loaded & cached.
    pub fn get_template(&self, name: &str) -> Option<Arc<Template>> {
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
        if let Some(tmpl) = templates::load(name) {
            let tmpl = Arc::new(tmpl);
            self.templates.lock().expect("Cache::templates lock poisoned")
                .insert(name.to_owned(), tmpl.clone());
            trace!("Template `{}` cached", name);
            return Some(tmpl);
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
