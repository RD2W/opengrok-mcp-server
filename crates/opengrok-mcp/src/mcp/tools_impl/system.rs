// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! System information tool handlers.

use opengrok_core::domain::OpengrokRepository;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

use crate::health::metrics;
use crate::mcp::OpengrokServer;
use crate::mcp::tools::NoParams;

pub async fn get_opengrok_version<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_opengrok_version().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_index_time<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_index_time().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn health_check<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.health_check().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}
