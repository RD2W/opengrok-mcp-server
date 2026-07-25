// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Streamable HTTP transport via axum + rmcp StreamableHttpService.

use std::sync::Arc;

use axum::{Router, routing::get};
use opengrok_core::application::OpengrokService;
use opengrok_core::domain::OpengrokRepository;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::config::Config;
use crate::health::{health_handler, metrics_handler, ready_handler};
use crate::mcp::OpengrokServer;

/// Runs the MCP server over Streamable HTTP with health/metrics endpoints.
pub async fn run_http<R: OpengrokRepository + Send + Sync + 'static>(
    config: &Config,
    service: OpengrokService<R>,
) -> anyhow::Result<()> {
    let service = Arc::new(service);

    // Factory creates a fresh OpengrokServer per MCP session,
    // all sharing the same OpengrokService (cache, rate-limiter, repo).
    let service_factory = {
        let svc = service.clone();
        move || Ok(OpengrokServer::new((*svc).clone()))
    };

    // Build server config: use defaults (localhost etc.) + user-configured hosts
    let mut server_config = StreamableHttpServerConfig::default();
    if !config.transport.allowed_hosts.is_empty() {
        server_config = server_config.with_allowed_hosts(&config.transport.allowed_hosts);
    }

    let mcp_service = StreamableHttpService::new(
        service_factory,
        Arc::new(LocalSessionManager::default()),
        server_config,
    );

    let health_path = config.transport.health_path.clone();
    let ready_path = config.transport.ready_path.clone();
    let metrics_path = config.transport.metrics_path.clone();
    let http_path = config.transport.http_path.clone();
    let bind_addr = config.transport.bind_addr.clone();

    let app = Router::new()
        .nest_service(&http_path, mcp_service)
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
