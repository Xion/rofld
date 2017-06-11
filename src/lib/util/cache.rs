//! Module implementing a thread-safe LRU cache.

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::atomic::{AtomicUsize, Ordering};

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
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self::with_hasher(capacity, RandomState::new())
    }
}

impl<K, V, S> ThreadSafeCache<K, V, S>
    where K: Eq + Hash, S: BuildHasher
{
    pub fn with_hasher(capacity: usize, hasher: S) -> Self {
        ThreadSafeCache{
            inner: Mutex::new(LruCache::with_hasher(capacity, hasher)),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    #[doc(hidden)]
    fn lock(&self) -> MutexGuard<LruCache<K, Arc<V>, S>> {
        self.inner.lock().expect("ThreadSafeCache mutex poisoned")
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
        ds.finish()
    }
}
