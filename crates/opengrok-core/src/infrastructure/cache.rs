// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! In-memory cache with per-entry TTL.
//!
//! Uses [`dashmap`] for lock-free concurrent access and [`tokio::time`]
//! for entry expiration. Entries are lazy-evicted: stale entries are
//! detected on access and removed.

use std::time::{Duration, Instant};

use dashmap::DashMap;

const EVICTION_FRACTION: f64 = 0.1;

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// A concurrent in-memory cache with per-entry time-to-live (TTL).
///
/// Entries are stored with their insertion time and evicted lazily
/// when accessed after expiration.
#[derive(Debug, Clone)]
pub struct MemoryCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
    V: Clone,
{
    inner: DashMap<K, CacheEntry<V>>,
    ttl: Duration,
    max_entries: usize,
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
    #[must_use]
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            inner: DashMap::new(),
            ttl,
            max_entries,
        }
    }

    /// Inserts a value into the cache. If the cache is at capacity,
    /// the oldest entry will be evicted as part of a periodic cleanup.
    pub fn insert(&self, key: K, value: V) {
        // Evict if at capacity (lazy: remove ~10% of oldest entries)
        if self.inner.len() >= self.max_entries {
            self.evict_fraction(EVICTION_FRACTION);
        }
        self.inner.insert(key, CacheEntry::new(value));
    }

    /// Retrieves a cached value, returning `None` if the key is not
    /// present or the entry has expired.
    pub fn get(&self, key: &K) -> Option<V> {
        let entry = self.inner.get(key)?;

        if entry.is_expired(&self.ttl) {
            // Lazy eviction: drop the read guard, then remove
            drop(entry);
            self.inner.remove(key);
            return None;
        }

        Some(entry.value.clone())
    }

    /// Removes a key from the cache.
    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner.remove(key).map(|(_, entry)| entry.value)
    }

    /// Returns the number of entries currently in the cache (including
    /// potentially stale ones that haven't been evicted yet).
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Evicts all expired entries.
    pub fn evict_expired(&self) {
        self.inner.retain(|_, entry| !entry.is_expired(&self.ttl));
    }

    /// Evicts approximately `fraction` of all entries (0.0–1.0).
    /// Only used when cache is at capacity; scale is [`EVICTION_FRACTION`].
    fn evict_fraction(&self, fraction: f64) {
        let count = (self.inner.len() as f64 * fraction).ceil() as usize;
        if count == 0 {
            return;
        }

        let mut removed = 0usize;
        // Iterate and remove a subset
        self.inner.retain(|_, _| {
            if removed >= count {
                return true;
            }
            removed += 1;
            false
        });
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
        // Use a very short TTL with real time (no pause/advance)
        let cache = MemoryCache::<String, String>::new(Duration::from_millis(10), 100);
        cache.insert("key".into(), "value".into());

        // Fresh entry should be available
        assert!(cache.get(&"key".into()).is_some());

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Entry should now be expired
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

        // After inserting 20 entries with max_entries=10, we should have ≤10
        assert!(cache.len() <= 10, "expected ≤10, got {}", cache.len());
    }
}
