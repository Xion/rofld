//! MOdule implementing cache for data needed to create image macros.
//! Currently, this includes image templates and fonts.

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering};

use lru_cache::LruCache;
use rusttype::Font;

use super::fonts;
use super::templates::{self, Template};


const DEFAULT_TEMPLATE_CAPACITY: usize = 128;
const DEFAULT_FONT_CAPACITY: usize = 16;


/// Cache for data used in rendering of image macros.
/// The cache is designed to be thread-safe.
pub struct Cache {
    templates: ThreadSafeCache<String, Template>,
    fonts: ThreadSafeCache<String, Font<'static>>,
}

impl Cache {
    #[inline]
    pub fn new() -> Self {
        Cache{
            templates: ThreadSafeCache::with_name(
                "Cache::templates", DEFAULT_TEMPLATE_CAPACITY),
            fonts: ThreadSafeCache::with_name(
                "Cache::fonts", DEFAULT_FONT_CAPACITY),
        }
    }
}

// TODO: use pub(crate) on these when available
impl Cache {
    #[inline]
    pub fn templates(&self) -> &ThreadSafeCache<String, Template> {
        &self.templates
    }

    #[inline]
    pub fn fonts(&self) -> &ThreadSafeCache<String, Font<'static>> {
        &self.fonts
    }
}

impl Cache {
    /// Get the image for a template of given name.
    /// If it doesn't exist in the cache, it will be loaded & cached.
    pub fn get_template(&self, name: &str) -> Option<Arc<Template>> {
        if let Some(img) = self.templates.get(name) {
            trace!("Cache hit for template `{}`", name);
            return Some(img);
        }
        debug!("Cache miss for template `{}`", name);
        self.load_template(name)
    }

    /// Load template of given name into the cache, even if it exists there already.
    pub fn load_template(&self, name: &str) -> Option<Arc<Template>> {
        if let Some(tmpl) = templates::load(name) {
            let tmpl = self.templates.put(name.to_owned(), tmpl);
            trace!("Template `{}` cached", name);
            return Some(tmpl);
        }
        None
    }

    /// Get the font with given name.
    /// If it doesn't exist in the cache, it will be loaded & cached.
    pub fn get_font(&self, name: &str) -> Option<Arc<Font<'static>>> {
        if let Some(font) = self.fonts.get(name) {
            trace!("Cache hit for font `{}`", name);
            return Some(font);
        }
        debug!("Cache miss for font `{}`", name);
        self.load_font(name)
    }

    /// Load font of given name into the cache, even if it exists there already.
    pub fn load_font(&self, name: &str) -> Option<Arc<Font<'static>>> {
        if let Some(font) = fonts::load(name) {
            let font = self.fonts.put(name.to_owned(), font);
            trace!("Font `{}` cached", name);
            return Some(font);
        }
        None
    }
}


/// A thread-safe cache of keys & cached values.
/// Actual values stored in the cache are Arc<V>'s.
///
/// This is a wrapper around LruCache that also counts various cache statistics,
/// like cache hits or cache misses.
pub struct ThreadSafeCache<K, V, S = RandomState>
    where K: Eq + Hash, S: BuildHasher
{
    inner: Mutex<LruCache<K, Arc<V>, S>>,
    name: Option<&'static str>,
    // Cache statistics.
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl<K: Eq + Hash, V> ThreadSafeCache<K, V> {
    #[allow(dead_code)]
    pub fn new(capacity: usize) -> Self {
        Self::with_hasher(None, capacity, RandomState::new())
    }

    pub fn with_name(name: &'static str, capacity: usize) -> Self {
        Self::with_hasher(Some(name), capacity, RandomState::new())
    }
}

impl<K, V, S> ThreadSafeCache<K, V, S>
    where K: Eq + Hash, S: BuildHasher
{
    pub fn with_hasher(name: Option<&'static str>, capacity: usize, hasher: S) -> Self {
        ThreadSafeCache{
            inner: Mutex::new(LruCache::with_hasher(capacity, hasher)),
            name: name,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    #[doc(hidden)]
    fn lock(&self) -> MutexGuard<LruCache<K, Arc<V>, S>> {
        self.inner.lock().expect(&format!(
            "{} mutex poisoned", self.name.unwrap_or("ThreadSafeCache")))
    }
}

// LruCache interface wrappers.
#[allow(dead_code)]
impl<K: Eq + Hash, V> ThreadSafeCache<K, V> {
    pub fn contains_key<Q>(&self, key: &Q) -> bool
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        let y = self.lock().contains_key(key);
        if !y { self.miss(); }
        y
    }

    pub fn get<Q>(&self, key: &Q) -> Option<Arc<V>>
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        match self.lock().get_mut(key) {
            Some(v) => { self.hit(); Some(v.clone()) }
            None => { self.miss(); None }
        }
    }

    /// Like insert(), except always returns the (Arc'd) value that's under the cached key.
    /// If it wasn't there before, it will be the new value just inserted (i.e. `v`).
    pub fn put(&self, k: K, v: V) -> Arc<V> {
        let value = Arc::new(v);
        self.lock().insert(k, value.clone()).unwrap_or_else(|| value)
    }

    pub fn insert(&self, k: K, v: V) -> Option<Arc<V>> {
        self.lock().insert(k, Arc::new(v))
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<Arc<V>>
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        match self.lock().remove(key) {
            r @ Some(_) => { self.hit(); r }
            r @ None => { self.miss(); r }
        }
    }

    pub fn capacity(&self) -> usize {
        self.lock().capacity()
    }

    pub fn set_capacity(&self, capacity: usize) {
        self.lock().set_capacity(capacity);
    }

    pub fn remove_lru(&self) -> Option<(K, Arc<V>)> {
        self.lock().remove_lru()
    }

    pub fn len(&self) -> usize {
        self.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    pub fn clear(&self) {
        self.lock().clear()
    }
}

// Incrementing the statistics' counters.
impl<K: Eq + Hash, V> ThreadSafeCache<K, V> {
    /// Increment the number of cache hits. Returns the new total.
    fn hit(&self) -> usize {
        let inc = 1;
        self.hits.fetch_add(inc, Ordering::Relaxed) + inc
    }

    /// Increment the number of cache misses. Returns the new total.
    fn miss(&self) -> usize {
        let inc = 1;
        self.misses.fetch_add(inc, Ordering::Relaxed) + inc
    }
}

// Getting counter values.
impl<K :Eq + Hash, V> ThreadSafeCache<K, V> {
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }
    pub fn misses(&self) -> usize {
        self.misses.load(Ordering::Relaxed)
    }
}

impl<K: Eq + Hash, V> fmt::Debug for ThreadSafeCache<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "ThreadSafeCache {{{}}}", self.name.unwrap_or("<unnamed>"))
    }
}
