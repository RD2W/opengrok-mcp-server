// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Token-bucket rate limiter.
//!
//! Uses the [`governor`] crate for a production-grade implementation
//! of the Generic Cell Rate Algorithm (GCRA). [`TokenBucket::acquire`]
//! is designed to be called before each OpenGrok API request.

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};

use crate::domain::DomainError;

// ---------------------------------------------------------------------------
// Token bucket
// ---------------------------------------------------------------------------

/// A concurrent token-bucket rate limiter.
///
/// Limits the number of requests to a maximum sustained rate with
/// a configurable burst size.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl TokenBucket {
    /// Creates a new token-bucket rate limiter.
    ///
    /// # Arguments
    /// * `requests_per_second` — sustained rate in req/s (must be ≥ 1).
    /// * `burst` — maximum burst size (must be ≥ 1).
    ///
    /// # Panics
    /// Panics if `requests_per_second` or `burst` is zero.
    #[must_use]
    pub fn new(requests_per_second: u32, burst: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second.max(1)).expect("requests_per_second must be >= 1"),
        )
        .allow_burst(NonZeroU32::new(burst.max(1)).expect("burst must be >= 1"));

        Self {
            limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// Acquires a token, blocking until one is available.
    ///
    /// Returns immediately if within limits; otherwise waits
    /// asynchronously.
    pub async fn acquire(&self) -> Result<(), DomainError> {
        self.limiter.until_ready().await;
        Ok(())
    }

    /// Checks whether a token is immediately available without blocking.
    #[must_use]
    pub fn check(&self) -> bool {
        self.limiter.check().is_ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_limiter() {
        let _bucket = TokenBucket::new(10, 20);
    }

    #[tokio::test]
    async fn acquire_returns_ok() {
        let bucket = TokenBucket::new(100, 200);
        let result = bucket.acquire().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_returns_true_when_tokens_available() {
        let bucket = TokenBucket::new(100, 200);
        assert!(bucket.check());
    }

    #[tokio::test]
    async fn multiple_acquires_work() {
        let bucket = TokenBucket::new(1000, 500);
        for _ in 0..100 {
            bucket.acquire().await.unwrap();
        }
    }

    #[test]
    fn clone_shares_state() {
        let a = TokenBucket::new(10, 10);
        let b = a.clone();
        assert!(a.check());
        assert!(b.check());
    }
}
