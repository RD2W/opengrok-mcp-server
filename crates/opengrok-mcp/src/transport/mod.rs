// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Transport layer.
//!
//! Dispatches between stdio and Streamable HTTP transports
//! based on configuration.

mod http;
mod stdio;

use opengrok_core::application::OpengrokService;
use opengrok_core::domain::OpengrokRepository;

use crate::config::Config;
use crate::mcp::OpengrokServer;

use self::http::run_http;
use self::stdio::run_stdio;

/// Runs the MCP server on the selected transport(s).
///
/// # Errors
/// Returns an error if transport binding fails.
pub async fn run_transport<R: OpengrokRepository + Send + Sync + 'static>(
    config: &Config,
    service: OpengrokService<R>,
) -> anyhow::Result<()> {
    tracing::info!(mode = %config.transport.mode, "starting transport");

    match config.transport.mode.as_str() {
        "stdio" => {
            let server = OpengrokServer::new(service);
            run_stdio(server).await?;
        }
        "http" => {
            run_http(config, service).await?;
        }
        "both" => {
            let http_config = config.clone();
            let http_service = service.clone();

            let http_handle = tokio::spawn(async move {
                if let Err(e) = run_http(&http_config, http_service).await {
                    tracing::error!(error = %e, "HTTP transport failed");
                }
            });

            let stdio_server = OpengrokServer::new(service);
            let stdio_handle = tokio::spawn(async move {
                if let Err(e) = run_stdio(stdio_server).await {
                    tracing::error!(error = %e, "stdio transport failed");
                }
            });

            tokio::select! {
                _ = http_handle => {}
                _ = stdio_handle => {}
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("shutting down...");
                }
            }
        }
        other => anyhow::bail!("unknown transport mode: '{other}' (expected stdio, http, or both)"),
    }

    Ok(())
}
