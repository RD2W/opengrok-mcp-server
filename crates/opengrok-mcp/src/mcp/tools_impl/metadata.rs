// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Project, group, and property tool handlers.

use opengrok_core::domain::OpengrokRepository;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

use crate::health::metrics;
use crate::mcp::OpengrokServer;
use crate::mcp::tools::*;

pub async fn list_indexed_projects<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.list_indexed_projects().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn list_all_projects<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.list_all_projects().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn list_groups<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.list_groups().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_group_projects<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetGroupProjectsParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_group_projects(&params.group).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn list_project_repos<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<ListProjectReposParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.list_project_repos(&params.project).await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_project_property<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetProjectPropertyParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server
        .service
        .get_project_property(&params.project, &params.name)
        .await
    {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_repo_property<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    Parameters(params): Parameters<GetRepoPropertyParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server
        .service
        .get_repo_property(&params.field, &params.repository)
        .await
    {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}

pub async fn get_suggest_config<R: OpengrokRepository + Send + Sync + 'static>(
    server: &OpengrokServer<R>,
    _params: Parameters<NoParams>,
) -> CallToolResult {
    metrics().record_tool_call();
    match server.service.get_suggest_config().await {
        Ok(text) => OpengrokServer::<R>::text_result(text),
        Err(e) => {
            metrics().record_tool_error();
            OpengrokServer::<R>::error_result(e.to_string())
        }
    }
}
