// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! MCP server struct, helpers, and ServerHandler implementation.

use std::borrow::Cow;
use std::sync::Arc;

use opengrok_core::application::OpengrokService;
use opengrok_core::domain::OpengrokRepository;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::*;

/// MCP server for OpenGrok code search.
///
/// Holds the application service and the auto-generated tool router.
#[derive(Debug)]
pub struct OpengrokServer<R: OpengrokRepository + Send + Sync + 'static> {
    pub(crate) service: Arc<OpengrokService<R>>,
    pub(crate) tool_router: ToolRouter<Self>,
}

// Manual Clone — ToolRouter and Arc are always Clone regardless of R
impl<R: OpengrokRepository + Send + Sync + 'static> Clone for OpengrokServer<R> {
    fn clone(&self) -> Self {
        Self {
            service: self.service.clone(),
            tool_router: self.tool_router.clone(),
        }
    }
}

impl<R: OpengrokRepository + Send + Sync + 'static> OpengrokServer<R> {
    /// Creates a new server wrapping the given service.
    #[must_use]
    pub fn new(service: OpengrokService<R>) -> Self {
        Self {
            service: Arc::new(service),
            tool_router: Self::tool_router(),
        }
    }

    pub(crate) fn to_projects(&self, maybe_project: &Option<String>) -> Vec<String> {
        match maybe_project {
            Some(p) => vec![p.clone()],
            None => vec![],
        }
    }

    pub(crate) fn text_result(text: String) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(text)])
    }

    pub(crate) fn error_result(msg: String) -> CallToolResult {
        CallToolResult::error(vec![ContentBlock::text(msg)])
    }
}

// ---------------------------------------------------------------------------
// ServerHandler implementation
// ---------------------------------------------------------------------------

#[rmcp::tool_handler]
impl<R: OpengrokRepository + Send + Sync + 'static> ServerHandler for OpengrokServer<R> {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.instructions = Some(
            "OpenGrok MCP server for AOSP-scale code search. \
             Use search_code for full-text queries, search_definition \
             to find where symbols are defined, search_references for \
             usage lookups, search_file_path for filename searches, \
             get_file_content to read file contents, get_history for \
             revision history, get_annotation for blame, \
             list_groups/list_indexed_projects/list_all_projects for \
             project navigation, get_suggest_config for suggester settings,\
             get_index_time for index freshness, get_opengrok_version \
             for API compatibility."
                .into(),
        );
        info
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28, ProtocolVersion::V_2025_11_25])
    }
}
