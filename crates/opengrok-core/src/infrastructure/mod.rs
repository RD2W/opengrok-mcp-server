// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Infrastructure implementations.
//!
//! Contains the OpenGrok HTTP client, TLS configuration builder,
//! in-memory cache, rate limiter, and result formatter. Each module
//! is independently testable.

pub mod cache;
pub mod client;
pub mod format;
pub mod rate_limit;
pub mod tls;
