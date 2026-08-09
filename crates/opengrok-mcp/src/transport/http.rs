// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Streamable HTTP transport via axum + rmcp StreamableHttpService.

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use opengrok_core::application::OpengrokService;
use opengrok_core::domain::OpengrokRepository;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use subtle::ConstantTimeEq;

use crate::config::Config;
use crate::health::{health_handler, metrics_handler, ready_handler};
use crate::mcp::OpengrokServer;

/// MCP token auth middleware: validates `Authorization: Bearer <token>`.
async fn mcp_token_auth(
    req: Request,
    next: Next,
    expected_token: Arc<str>,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let Some(provided) = auth_header else {
        tracing::warn!("MCP token auth: missing Authorization header");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if provided.as_bytes().ct_ne(expected_token.as_bytes()).into() {
        tracing::warn!("MCP token auth: invalid token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

/// Runs the MCP server over Streamable HTTP with health/metrics endpoints.
pub async fn run_http<R: OpengrokRepository + Send + Sync + 'static>(
    config: &Config,
    service: OpengrokService<R>,
) -> anyhow::Result<()> {
    let service = Arc::new(service);

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

    let mut mcp_routes = Router::new().nest_service(&http_path, mcp_service);

    if !config.transport.mcp_auth_token.is_empty() {
        let token: Arc<str> = config.transport.mcp_auth_token.clone().into();
        let middleware_fn = move |req: Request, next: Next| {
            let token = Arc::clone(&token);
            mcp_token_auth(req, next, token)
        };
        mcp_routes = mcp_routes.layer(middleware::from_fn(middleware_fn));
        tracing::info!("MCP token auth enabled");
    }

    let app = Router::new()
        .merge(mcp_routes)
        .route(&health_path, get(health_handler))
        .route(&ready_path, get(ready_handler))
        .route(&metrics_path, get(metrics_handler));

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

    fn make_app(token: &str) -> Router {
        let t: Arc<str> = token.into();
        let middleware_fn = move |req: Request, next: Next| {
            let t = Arc::clone(&t);
            mcp_token_auth(req, next, t)
        };
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn(middleware_fn))
    }

    #[test]
    fn constant_time_eq_matches() {
        assert!(bool::from(b"abc".ct_eq(b"abc")));
        assert!(bool::from(b"".ct_eq(b"")));
    }

    #[test]
    fn constant_time_eq_mismatches() {
        assert!(bool::from(b"abc".ct_ne(b"abd")));
        assert!(bool::from(b"abc".ct_ne(b"ab")));
        assert!(bool::from(b"abc".ct_ne(b"ABC")));
    }

    #[tokio::test]
    async fn correct_token_passes() {
        use axum::body::Body;
        let app = make_app("secret");

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
        let app = make_app("secret");

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
        let app = make_app("secret");

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
