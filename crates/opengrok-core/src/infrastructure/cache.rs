// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! In-memory cache with per-entry TTL and LRU eviction.
//!
//! Uses the [`lru`] crate for access-recency-aware eviction and a
//! [`std::sync::Mutex`] for thread-safe interior mutability.  Entries
//! carry insertion timestamps and are lazy-evicted on access when
//! their TTL has elapsed.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lru::LruCache;

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// A thread-safe in-memory cache with LRU eviction and per-entry TTL.
///
/// Entries are stored with their insertion time and evicted lazily
/// when accessed after expiration.  When the cache is at capacity the
/// least-recently-used entry (by access) is dropped before inserting
/// a new one.
#[derive(Debug, Clone)]
pub struct MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    inner: Arc<Mutex<LruCache<K, CacheEntry<V>>>>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: &Duration) -> bool {
        self.inserted_at.elapsed() >= *ttl
    }
}

impl<K, V> MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    /// Creates a new cache with the given TTL and maximum entry count.
    ///
    /// Panics if `max_entries` is 0.
    #[must_use]
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        let cap = NonZeroUsize::new(max_entries).expect("max_entries must be > 0");
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(cap))),
            ttl,
        }
    }

    /// Inserts a value into the cache.  If the cache is at capacity the
    /// least-recently-used entry is evicted first.
    pub fn insert(&self, key: K, value: V) {
        self.inner
            .lock()
            .expect("cache lock poisoned")
            .put(key, CacheEntry::new(value));
    }

    /// Retrieves a cached value.  Returns `None` if the key is absent
    /// or the entry has expired.  After retrieving, the entry is promoted
    /// as the most-recently-used.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.inner.lock().expect("cache lock poisoned");

        let entry = cache.get(key)?;

        if entry.is_expired(&self.ttl) {
            cache.pop(key);
            return None;
        }

        Some(entry.value.clone())
    }

    /// Removes a key from the cache, returning its value if present.
    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner
            .lock()
            .expect("cache lock poisoned")
            .pop(key)
            .map(|entry| entry.value)
    }

    /// Returns the number of entries currently in the cache (including
    /// potentially stale ones that haven't been evicted yet).
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("cache lock poisoned").len()
    }

    /// Returns `true` if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evicts all expired entries.
    pub fn evict_expired(&self) {
        let mut cache = self.inner.lock().expect("cache lock poisoned");
        let ttl = self.ttl;

        let mut expired_keys = Vec::new();
        for (key, entry) in cache.iter() {
            if entry.is_expired(&ttl) {
                expired_keys.push(key.clone());
            }
        }
        for key in expired_keys {
            cache.pop(&key);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_retrieve() {
        let cache = MemoryCache::<String, String>::new(Duration::from_secs(60), 100);
        cache.insert("key".into(), "value".into());
        assert_eq!(cache.get(&"key".into()), Some("value".into()));
    }

    #[test]
    fn missing_key_returns_none() {
        let cache = MemoryCache::<String, String>::new(Duration::from_secs(60), 100);
        assert_eq!(cache.get(&"missing".into()), None);
    }

    #[test]
    fn remove_returns_value() {
        let cache = MemoryCache::<String, i32>::new(Duration::from_secs(60), 100);
        cache.insert("a".into(), 42);
        assert_eq!(cache.remove(&"a".into()), Some(42));
        assert_eq!(cache.get(&"a".into()), None);
    }

    #[test]
    fn len_reflects_inserts() {
        let cache = MemoryCache::<i32, i32>::new(Duration::from_secs(60), 100);
        assert_eq!(cache.len(), 0);
        cache.insert(1, 10);
        assert_eq!(cache.len(), 1);
        cache.insert(2, 20);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn is_empty() {
        let cache = MemoryCache::<i32, i32>::new(Duration::from_secs(60), 100);
        assert!(cache.is_empty());
        cache.insert(1, 10);
        assert!(!cache.is_empty());
    }

    #[tokio::test]
    async fn entry_expires_after_ttl() {
        let cache = MemoryCache::<String, String>::new(Duration::from_millis(10), 100);
        cache.insert("key".into(), "value".into());

        assert!(cache.get(&"key".into()).is_some());

        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(cache.get(&"key".into()).is_none());
        assert!(cache.is_empty(), "expired entry should be evicted");
    }

    #[tokio::test]
    async fn evict_expired_removes_stale_entries() {
        let cache = MemoryCache::<i32, i32>::new(Duration::from_millis(10), 100);
        cache.insert(1, 10);
        cache.insert(2, 20);

        tokio::time::sleep(Duration::from_millis(50)).await;

        cache.evict_expired();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn capacity_limit_triggers_eviction() {
        let cache = MemoryCache::<i32, i32>::new(Duration::from_secs(3600), 10);

        for i in 0..20 {
            cache.insert(i, i * 10);
        }

        assert!(cache.len() <= 10, "expected ≤10, got {}", cache.len());
    }

    #[test]
    fn lru_eviction_respects_access_recency() {
        let cache = MemoryCache::<i32, i32>::new(Duration::from_secs(3600), 3);

        cache.insert(1, 10);
        cache.insert(2, 20);
        cache.insert(3, 30);

        // Access key 1 — promotes it as most-recently-used
        assert_eq!(cache.get(&1), Some(10));

        // Insert key 4 — should evict key 2 (least-recently-used)
        cache.insert(4, 40);

        assert_eq!(
            cache.get(&1),
            Some(10),
            "key 1 was accessed, should survive"
        );
        assert_eq!(cache.get(&3), Some(30), "key 3 should survive");
        assert_eq!(cache.get(&4), Some(40), "key 4 was just inserted");
        assert_eq!(cache.get(&2), None, "key 2 should be evicted (LRU)");
    }
}
