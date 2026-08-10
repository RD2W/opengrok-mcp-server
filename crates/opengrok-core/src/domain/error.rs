// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Domain error types.

// ---------------------------------------------------------------------------
// Domain errors
// ---------------------------------------------------------------------------

/// Unified error type for the domain layer.
///
/// Uses `#[error(transparent)] #[from]` so that `?` propagates
/// underlying errors (`reqwest::Error`, `serde_json::Error`, etc.)
/// without explicit `.map_err()` at every call site.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Search requires at least one of: full, def, symbol, path, hist.
    #[error("opengrok: search requires at least one of: full, def, symbol, path, hist")]
    EmptyQuery,

    /// An invalid project name was provided.
    #[error("opengrok: invalid project name: {0}")]
    InvalidProject(String),

    /// OpenGrok returned a non-2xx response.
    #[error("opengrok: HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },

    /// Network/transport error (DNS, TLS, timeout, connection refused).
    #[error("opengrok: network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON decode error (malformed response).
    #[error("opengrok: decode error: {0}")]
    Decode(#[from] serde_json::Error),

    /// TLS or certificate error.
    #[error("opengrok: TLS/cert error: {0}")]
    Tls(String),

    /// Cache operation failed.
    #[error(transparent)]
    Cache(#[from] CacheError),

    /// Rate limit exceeded.
    #[error(transparent)]
    RateLimit(#[from] RateLimitError),

    /// The requested operation is not yet implemented.
    #[error("opengrok: not yet implemented")]
    NotImplemented,
}

/// Cache-related errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    #[error("cache capacity exceeded")]
    CapacityExceeded,
}

/// Rate-limit errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded, retry after {retry_after_secs}s")]
    Exceeded { retry_after_secs: u64 },
}
