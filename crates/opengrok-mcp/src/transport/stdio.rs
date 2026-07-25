// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Stdio transport.

use opengrok_core::domain::OpengrokRepository;
use rmcp::ServiceExt;
use rmcp::transport::io;

use crate::mcp::OpengrokServer;

/// Runs the MCP server over stdio (for docker exec usage).
pub async fn run_stdio<R: OpengrokRepository + Send + Sync + 'static>(
    server: OpengrokServer<R>,
) -> anyhow::Result<()> {
    tracing::info!("starting stdio transport");
    let handle = server.serve(io::stdio()).await?;
    handle.waiting().await?;
    Ok(())
}
