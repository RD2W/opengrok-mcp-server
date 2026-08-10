// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! History and annotation tool handlers.

use opengrok_core::domain::{HistoryRequest, OpengrokRepository};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

use crate::health::metrics;
use crate::mcp::OpengrokServer;
use crate::mcp::tools::*;

pub async fn get_history<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetHistoryParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    let req = HistoryRequest {
        path: params.path,
        start: params.start,
        max: params.max,
        with_files: params.with_files,
    };
    match server.service.get_history(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_annotation<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetAnnotationParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_annotation(&params.path).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}
