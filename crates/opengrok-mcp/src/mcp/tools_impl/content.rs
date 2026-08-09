// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Content and suggest tool handlers.

use opengrok_core::domain::{OpengrokRepository, SuggestRequest};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

use crate::health::metrics;
use crate::mcp::OpengrokServer;
use crate::mcp::tools::*;

pub async fn suggest<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SuggestParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    let req = SuggestRequest {
        projects: vec![params.project],
        field: params.field,
        caret: params.caret,
        full: params.full,
        defs: params.defs,
        refs: params.refs,
        path: params.path,
        hist: None,
        file_type: params.file_type,
    };
    match server.service.suggest(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_file_content<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetFileContentParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server
        .service
        .get_file_content(&params.project, &params.path)
        .await
    {
        Ok(file) => OpengrokServer::<R>::text_result(file.text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_file_definitions<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetFileDefinitionsParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_file_definitions(&params.path).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_file_genre<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetFileGenreParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_file_genre(&params.path).await {
        Ok(genre) => OpengrokServer::<R>::text_result(format!("{genre:?}")),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn list_directory<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<ListDirectoryParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.list_directory(&params.path).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn list_project_files<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<ListProjectFilesParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    let path = format!(
        "/{}/{}",
        params.project,
        params.path.trim_start_matches('/')
    );
    match server.service.list_directory(&path).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}
