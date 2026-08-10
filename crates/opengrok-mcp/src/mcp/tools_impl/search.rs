// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Search tool handlers. Delegated from `mcp/mod.rs`.

use opengrok_core::domain::{OpengrokRepository, SearchRequest, SortOrder};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

use crate::health::metrics;
use crate::mcp::OpengrokServer;
use crate::mcp::tools::*;

pub async fn search_code<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SearchCodeParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let req = SearchRequest {
        full: Some(params.query),
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        ..Default::default()
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn search_definition<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SearchDefinitionParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let req = SearchRequest {
        def: Some(params.symbol),
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        ..Default::default()
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn search_references<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SearchReferencesParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let req = SearchRequest {
        symbol: Some(params.symbol),
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        ..Default::default()
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn search_file_path<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SearchFilePathParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let req = SearchRequest {
        path: Some(params.path),
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        ..Default::default()
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn search_history<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<SearchHistoryParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let req = SearchRequest {
        hist: Some(params.hist),
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        ..Default::default()
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn advanced_search<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<AdvancedSearchParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    metrics().record_search();
    let sort = params.sort.as_deref().and_then(|s| match s {
        s if s == SortOrder::Relevancy.as_query_value() => Some(SortOrder::Relevancy),
        s if s == SortOrder::FullPath.as_query_value() => Some(SortOrder::FullPath),
        s if s == SortOrder::LastModTime.as_query_value() => Some(SortOrder::LastModTime),
        _ => None,
    });

    let req = SearchRequest {
        full: params.full,
        def: params.def,
        symbol: params.symbol,
        path: params.path,
        hist: params.hist,
        file_type: params.file_type,
        projects: server.to_projects(&params.project),
        max_results: Some(params.max_results),
        start: params.start.map(|v| v as u32),
        max_hits_per_file: params.max_hits_per_file,
        sort,
    };
    match server.service.search(req).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}
