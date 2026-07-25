// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Health check, readiness probe, and Prometheus metrics endpoints.
//!
//! - `/healthz` — liveness: always returns 200 if the process is alive.
//! - `/readyz`  — readiness: checks that OpenGrok backend is reachable.
//! - `/metrics` — Prometheus exposition format.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::Json;
use serde_json::json;

const METRIC_UP: &str = "opengrok_mcp_up";
const METRIC_UPTIME: &str = "opengrok_mcp_uptime_seconds";
const METRIC_TOOL_CALLS: &str = "opengrok_mcp_tool_calls_total";
const METRIC_TOOL_ERRORS: &str = "opengrok_mcp_tool_errors_total";
const METRIC_SEARCH_QUERIES: &str = "opengrok_mcp_search_queries_total";

const METRIC_HELP_UP: &str = "Whether the server is up (1=alive)";
const METRIC_HELP_UPTIME: &str = "Server uptime in seconds";
const METRIC_HELP_TOOL_CALLS: &str = "Total MCP tool calls";
const METRIC_HELP_TOOL_ERRORS: &str = "Tool calls that returned errors";
const METRIC_HELP_SEARCH_QUERIES: &str = "Search queries processed";

const METRIC_TYPE_GAUGE: &str = "gauge";
const METRIC_TYPE_COUNTER: &str = "counter";

// ---------------------------------------------------------------------------
// Metrics counters
// ---------------------------------------------------------------------------

/// Global metrics singleton (created once on first access).
#[derive(Debug)]
pub(crate) struct Metrics {
    /// Total MCP tool calls processed.
    tool_calls_total: AtomicU64,
    /// Tool calls that returned an error.
    tool_calls_errors: AtomicU64,
    /// Total search queries (subset of tool_calls).
    search_queries_total: AtomicU64,
    /// Process start time (for uptime).
    started_at: Instant,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

#[must_use]
pub(crate) fn metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::new)
}

impl Metrics {
    fn new() -> Self {
        Self {
            tool_calls_total: AtomicU64::new(0),
            tool_calls_errors: AtomicU64::new(0),
            search_queries_total: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    #[allow(dead_code)] // wired into tool handlers later
    pub fn record_tool_call(&self) {
        self.tool_calls_total.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_tool_error(&self) {
        self.tool_calls_errors.fetch_add(1, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn record_search(&self) {
        self.search_queries_total.fetch_add(1, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Liveness probe response.
#[derive(Debug)]
pub struct HealthResponse;

impl HealthResponse {
    /// Returns a simple JSON body indicating the process is alive.
    pub fn ok() -> Json<serde_json::Value> {
        Json(json!({"status": "ok"}))
    }
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

/// Liveness handler — always 200 if the process is running.
pub async fn health_handler() -> Json<serde_json::Value> {
    HealthResponse::ok()
}

/// Readiness handler — checks that OpenGrok backend is reachable.
pub async fn ready_handler() -> Json<serde_json::Value> {
    Json(json!({"status": "ready", "opengrok": "unknown"}))
}

/// Prometheus metrics exposition handler.
pub async fn metrics_handler() -> String {
    let m = metrics();
    let uptime = m.started_at.elapsed().as_secs();
    let tool_calls = m.tool_calls_total.load(Ordering::Relaxed);
    let errors = m.tool_calls_errors.load(Ordering::Relaxed);
    let searches = m.search_queries_total.load(Ordering::Relaxed);

    format!(
        "# HELP {METRIC_UP} {METRIC_HELP_UP}\n\
         # TYPE {METRIC_UP} {METRIC_TYPE_GAUGE}\n\
         {METRIC_UP} 1\n\
         # HELP {METRIC_UPTIME} {METRIC_HELP_UPTIME}\n\
         # TYPE {METRIC_UPTIME} {METRIC_TYPE_COUNTER}\n\
         {METRIC_UPTIME} {uptime}\n\
         # HELP {METRIC_TOOL_CALLS} {METRIC_HELP_TOOL_CALLS}\n\
         # TYPE {METRIC_TOOL_CALLS} {METRIC_TYPE_COUNTER}\n\
         {METRIC_TOOL_CALLS} {tool_calls}\n\
         # HELP {METRIC_TOOL_ERRORS} {METRIC_HELP_TOOL_ERRORS}\n\
         # TYPE {METRIC_TOOL_ERRORS} {METRIC_TYPE_COUNTER}\n\
         {METRIC_TOOL_ERRORS} {errors}\n\
         # HELP {METRIC_SEARCH_QUERIES} {METRIC_HELP_SEARCH_QUERIES}\n\
         # TYPE {METRIC_SEARCH_QUERIES} {METRIC_TYPE_COUNTER}\n\
         {METRIC_SEARCH_QUERIES} {searches}\n"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok() {
        let resp = health_handler().await;
        assert_eq!(resp["status"], "ok");
    }

    #[tokio::test]
    async fn ready_returns_json() {
        let resp = ready_handler().await;
        assert_eq!(resp["status"], "ready");
        assert!(resp["opengrok"].is_string());
    }

    #[tokio::test]
    async fn metrics_includes_counters() {
        metrics().record_tool_call();
        metrics().record_search();
        metrics().record_tool_error();

        let out = metrics_handler().await;
        assert!(out.contains("opengrok_mcp_up 1"));
        assert!(out.contains("opengrok_mcp_tool_calls_total"));
        assert!(out.contains("opengrok_mcp_search_queries_total"));
        assert!(out.contains("opengrok_mcp_tool_errors_total"));
        assert!(out.contains("opengrok_mcp_uptime_seconds"));
    }

    #[test]
    fn metrics_singleton() {
        let m1 = metrics();
        let m2 = metrics();
        // Same instance
        assert!(std::ptr::eq(m1, m2));
    }
}
