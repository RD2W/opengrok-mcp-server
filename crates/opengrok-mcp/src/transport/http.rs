// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Streamable HTTP transport via axum + rmcp StreamableHttpService.

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::get;
use opengrok_core::application::OpengrokService;
use opengrok_core::domain::OpengrokRepository;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};

use crate::config::Config;
use crate::health::{health_handler, metrics_handler, ready_handler};
use crate::mcp::OpengrokServer;

/// MCP token auth middleware: validates `Authorization: Bearer <token>`.
///
/// When `mcp_token` is `Some`, every request to the MCP path is checked.
/// Requests without a matching `Authorization: Bearer` header receive
/// HTTP 401 with a plain-text body.  Comparison uses a constant-time
/// helper to eliminate timing side-channels.
///
/// When `mcp_token` is `None`, all requests pass through (no auth).
async fn mcp_token_auth(
    mcp_token: Option<String>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected = match mcp_token {
        Some(t) => t,
        None => return Ok(next.run(req).await),
    };

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(provided) if constant_time_eq(provided, &expected) => Ok(next.run(req).await),
        _ => {
            let body = axum::body::Body::from("Unauthorized\n");
            Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(body)
                .unwrap())
        }
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for i in 0..a.len() {
        acc |= a[i] ^ b[i];
    }
    acc == 0
}

/// Runs the MCP server over Streamable HTTP with health/metrics endpoints.
pub async fn run_http<R: OpengrokRepository + Send + Sync + 'static>(
    config: &Config,
    service: OpengrokService<R>,
) -> anyhow::Result<()> {
    let service = Arc::new(service);

    // Factory creates a fresh OpengrokServer per request,
    // all sharing the same OpengrokService (cache, rate-limiter, repo).
    // NeverSessionManager: stateless, no session tracking (2026-07-28 protocol).
    let service_factory = {
        let svc = service.clone();
        move || Ok(OpengrokServer::new((*svc).clone()))
    };

    let mut server_config = StreamableHttpServerConfig::default().with_legacy_session_mode(false);
    if !config.transport.allowed_hosts.is_empty() {
        server_config = server_config.with_allowed_hosts(&config.transport.allowed_hosts);
    }

    let mcp_service = StreamableHttpService::new(
        service_factory,
        Arc::new(NeverSessionManager::default()),
        server_config,
    );

    let health_path = config.transport.health_path.clone();
    let ready_path = config.transport.ready_path.clone();
    let metrics_path = config.transport.metrics_path.clone();
    let http_path = config.transport.http_path.clone();
    let bind_addr = config.transport.bind_addr.clone();
    let mcp_token = config.transport.mcp_token.clone();

    let mcp_token_for_middleware = mcp_token.clone();
    let app = Router::new()
        .nest_service(&http_path, mcp_service)
        .route(&health_path, get(health_handler))
        .route(&ready_path, get(ready_handler))
        .route(&metrics_path, get(metrics_handler))
        .layer(axum::middleware::from_fn(move |req, next| {
            let token = mcp_token_for_middleware.clone();
            async move { mcp_token_auth(token, req, next).await }
        }));

    if let Some(_t) = &mcp_token {
        tracing::info!("MCP token auth enabled");
    }

    tracing::info!(%bind_addr, %http_path, "starting Streamable HTTP transport");

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[test]
    fn constant_time_eq_matches() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn constant_time_eq_mismatches() {
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
        assert!(!constant_time_eq("abc", "abcd"));
        assert!(!constant_time_eq("abc", "ABC"));
    }

    #[tokio::test]
    async fn no_token_auth_passes() {
        use axum::body::Body;
        let app =
            Router::new()
                .route("/test", get(|| async { "ok" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let token: Option<String> = None;
                    async move { mcp_token_auth(token, req, next).await }
                }));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn correct_token_passes() {
        use axum::body::Body;
        let app =
            Router::new()
                .route("/test", get(|| async { "ok" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let token = Some("secret".to_string());
                    async move { mcp_token_auth(token, req, next).await }
                }));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("Authorization", "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn wrong_token_returns_401() {
        use axum::body::Body;
        let app =
            Router::new()
                .route("/test", get(|| async { "ok" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let token = Some("secret".to_string());
                    async move { mcp_token_auth(token, req, next).await }
                }));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("Authorization", "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn missing_header_returns_401() {
        use axum::body::Body;
        let app =
            Router::new()
                .route("/test", get(|| async { "ok" }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let token = Some("secret".to_string());
                    async move { mcp_token_auth(token, req, next).await }
                }));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
