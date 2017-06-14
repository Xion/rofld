//! Module implementing a thread-safe LRU cache.

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use antidote::Mutex;
use lru_cache::LruCache;


/// A thread-safe cache of keys & cached values.
/// Actual values stored in the cache are `Arc<V>'`s.
///
/// This is a wrapper around `LruCache` that also counts various cache statistics,
/// like cache hits or cache misses.
pub struct ThreadSafeCache<K, V, S = RandomState>
    where K: Eq + Hash, S: BuildHasher
{
    inner: Mutex<LruCache<K, Arc<V>, S>>,
    // Cache statistics.
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl<K: Eq + Hash, V> ThreadSafeCache<K, V> {
    /// Create the cache with given capacity.
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self::with_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> ThreadSafeCache<K, V, S>
    where K: Eq + Hash, S: BuildHasher
{
    /// Create the cache with custom hasher and given capacity.
    pub fn with_hasher(capacity: usize, hasher: S) -> Self {
        ThreadSafeCache{
            inner: Mutex::new(LruCache::with_hasher(capacity, hasher)),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }
}

// LruCache interface wrappers.
#[allow(dead_code)]
impl<K: Eq + Hash, V> ThreadSafeCache<K, V> {
    /// Check if the cache contains given key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        let y = self.inner.lock().contains_key(key);
        if !y { self.miss(); }
        y
    }

    /// Get the element corresponding to given key if it's present in the cache.
    pub fn get<Q>(&self, key: &Q) -> Option<Arc<V>>
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        match self.inner.lock().get_mut(key) {
            Some(v) => { self.hit(); Some(v.clone()) }
            None => { self.miss(); None }
        }
    }

    /// Put an item into cache under given key.
    ///
    /// This is like insert(), except it always returns the (`Arc`'d) value
    /// that's under the cached key.
    /// If it wasn't there before, it will be the new value just inserted (i.e. `v`).
    pub fn put(&self, k: K, v: V) -> Arc<V> {
        let value = Arc::new(v);
        self.inner.lock().insert(k, value.clone()).unwrap_or_else(|| value)
    }

    /// Insert an item into the cache under given key.
    ///
    /// If the key is already present in the cache, returns its corresponding value.
    pub fn insert(&self, k: K, v: V) -> Option<Arc<V>> {
        self.inner.lock().insert(k, Arc::new(v))
    }

    /// Removes a key from the cache, if present, and returns its value.
    pub fn remove<Q>(&self, key: &Q) -> Option<Arc<V>>
        where K: Borrow<Q>, Q: ?Sized + Eq + Hash
    {
        match self.inner.lock().remove(key) {
            r @ Some(_) => { self.hit(); r }
            r @ None => { self.miss(); r }
        }
    }

    /// Cache capacity.
    pub fn capacity(&self) -> usize {
        self.inner.lock().capacity()
    }

    /// Set the capacity of the cache.
    ///
    /// If the new capacity is smaller than current size of the cache,
    /// elements will be removed from it in the LRU manner.
    pub fn set_capacity(&self, capacity: usize) {
        self.inner.lock().set_capacity(capacity);
    }

    /// Remove the least recently used element from the cache.
    pub fn remove_lru(&self) -> Option<(K, Arc<V>)> {
        self.inner.lock().remove_lru()
    }

    /// Current size of the cache.
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// Remove all elements from the cache.
    pub fn clear(&self) {
        self.inner.lock().clear()
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
    /// Returns the number of cache hits.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }

    /// Returns the number of cache misses.
    pub fn misses(&self) -> usize {
        self.misses.load(Ordering::Relaxed)
    }
}

impl<K: Eq + Hash, V> fmt::Debug for ThreadSafeCache<K, V> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut ds = fmt.debug_struct("ThreadSafeCache");
        if let Ok(inner) = self.inner.try_lock() {
            ds.field("capacity", &inner.capacity());
            ds.field("len", &inner.len());
        }
        ds.field("hits", &self.hits());
        ds.field("misses", &self.misses());
        ds.finish()
    }
}
