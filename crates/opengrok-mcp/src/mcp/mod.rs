// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! MCP server implementation.
//!
//! Defines [`OpengrokServer<R>`] — the MCP server struct with all 25
//! tools, registered via rmcp's `#[tool_router]` / `#[tool_handler]`
//! macros. Generic over the repository implementation so that tests
//! can inject a mock.

pub mod tools;

use std::borrow::Cow;
use std::sync::Arc;

use opengrok_core::application::OpengrokService;
use opengrok_core::domain::*;
use rmcp::handler::server::ServerHandler;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::tool;
use rmcp::tool_handler;
use rmcp::tool_router;

use self::tools::*;

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

/// MCP server for OpenGrok code search.
///
/// Holds the application service and the auto-generated tool router.
#[derive(Debug)]
pub struct OpengrokServer<R: OpengrokRepository + Send + Sync + 'static> {
    service: Arc<OpengrokService<R>>,
    tool_router: ToolRouter<Self>,
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

    fn to_projects(&self, maybe_project: &Option<String>) -> Vec<String> {
        maybe_project.clone().map(|p| vec![p]).unwrap_or_default()
    }

    fn text_result(text: String) -> CallToolResult {
        CallToolResult::success(vec![ContentBlock::text(text)])
    }

    fn error_result(msg: String) -> CallToolResult {
        CallToolResult::error(vec![ContentBlock::text(msg)])
    }
}

// ---------------------------------------------------------------------------
// Tool definitions (25 tools)
// ---------------------------------------------------------------------------

#[tool_router]
impl<R: OpengrokRepository + Send + Sync + 'static> OpengrokServer<R> {
    // 1. search_code
    #[tool(
        description = "Full-text search in OpenGrok code index (Lucene syntax). Use for finding code by keywords, function names, strings, etc."
    )]
    async fn search_code(
        &self,
        Parameters(params): Parameters<SearchCodeParams>,
    ) -> CallToolResult {
        let req = SearchRequest {
            full: Some(params.query),
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            ..Default::default()
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 2. search_definition
    #[tool(
        description = "Find where a symbol is defined (classes, functions, variables). Returns file paths and line numbers of definitions."
    )]
    async fn search_definition(
        &self,
        Parameters(params): Parameters<SearchDefinitionParams>,
    ) -> CallToolResult {
        let req = SearchRequest {
            def: Some(params.symbol),
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            ..Default::default()
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 3. search_references
    #[tool(description = "Find all references/uses of a symbol across the codebase.")]
    async fn search_references(
        &self,
        Parameters(params): Parameters<SearchReferencesParams>,
    ) -> CallToolResult {
        let req = SearchRequest {
            symbol: Some(params.symbol),
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            ..Default::default()
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 4. search_file_path
    #[tool(
        description = "Search for files by path pattern (glob-style). E.g. 'MainActivity.java' or '*.xml'."
    )]
    async fn search_file_path(
        &self,
        Parameters(params): Parameters<SearchFilePathParams>,
    ) -> CallToolResult {
        let req = SearchRequest {
            path: Some(params.path),
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            ..Default::default()
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 5. search_history
    #[tool(description = "Search file history/changelog for matching entries.")]
    async fn search_history(
        &self,
        Parameters(params): Parameters<SearchHistoryParams>,
    ) -> CallToolResult {
        let req = SearchRequest {
            hist: Some(params.hist),
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            ..Default::default()
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 6. advanced_search
    #[tool(
        description = "Advanced search with full field access (full-text, definitions, references, path, history, type filter, pagination, sorting)."
    )]
    async fn advanced_search(
        &self,
        Parameters(params): Parameters<AdvancedSearchParams>,
    ) -> CallToolResult {
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
            projects: self.to_projects(&params.project),
            max_results: Some(params.max_results),
            start: params.start.map(|v| v as u32),
            max_hits_per_file: params.max_hits_per_file,
            sort,
        };
        match self.service.search(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 7. suggest
    #[tool(
        description = "Get autocomplete suggestions for a partial query in the given project and field."
    )]
    async fn suggest(&self, Parameters(params): Parameters<SuggestParams>) -> CallToolResult {
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
        match self.service.suggest(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 8. get_file_content
    #[tool(
        description = "Retrieve the raw content of a file from OpenGrok. Returns the file text."
    )]
    async fn get_file_content(
        &self,
        Parameters(params): Parameters<GetFileContentParams>,
    ) -> CallToolResult {
        match self
            .service
            .get_file_content(&params.project, &params.path)
            .await
        {
            Ok(file) => Self::text_result(file.text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 9. get_file_definitions
    #[tool(
        description = "List all definitions (functions, classes, methods, variables) found within a specific file."
    )]
    async fn get_file_definitions(
        &self,
        Parameters(params): Parameters<GetFileDefinitionsParams>,
    ) -> CallToolResult {
        match self.service.get_file_definitions(&params.path).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 10. get_file_genre
    #[tool(
        description = "Get the analyzer-detected genre of a file (PLAIN, XREFABLE, IMAGE, DATA, HTML)."
    )]
    async fn get_file_genre(
        &self,
        Parameters(params): Parameters<GetFileGenreParams>,
    ) -> CallToolResult {
        match self.service.get_file_genre(&params.path).await {
            Ok(genre) => Self::text_result(format!("{genre:?}")),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 11. list_directory
    #[tool(description = "List the contents of a directory in the source tree.")]
    async fn list_directory(
        &self,
        Parameters(params): Parameters<ListDirectoryParams>,
    ) -> CallToolResult {
        match self.service.list_directory(&params.path).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 12. list_indexed_projects
    #[tool(description = "List all indexed (searchable) projects in OpenGrok.")]
    async fn list_indexed_projects(
        &self,
        Parameters(_params): Parameters<NoParams>,
    ) -> CallToolResult {
        match self.service.list_indexed_projects().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 13. list_all_projects
    #[tool(description = "List all configured projects in OpenGrok (including non-indexed).")]
    async fn list_all_projects(&self, Parameters(_params): Parameters<NoParams>) -> CallToolResult {
        match self.service.list_all_projects().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 14. get_history
    #[tool(description = "Get the revision history (commit log) for a file. Paginated.")]
    async fn get_history(
        &self,
        Parameters(params): Parameters<GetHistoryParams>,
    ) -> CallToolResult {
        let req = HistoryRequest {
            path: params.path,
            start: params.start,
            max: params.max,
            with_files: params.with_files,
        };
        match self.service.get_history(req).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 15. get_annotation
    #[tool(
        description = "Get per-line annotation (blame/git-blame) for a file. Shows revision and author for each line."
    )]
    async fn get_annotation(
        &self,
        Parameters(params): Parameters<GetAnnotationParams>,
    ) -> CallToolResult {
        match self.service.get_annotation(&params.path).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 16. list_groups
    #[tool(description = "List all configured project groups in OpenGrok.")]
    async fn list_groups(&self, Parameters(_params): Parameters<NoParams>) -> CallToolResult {
        match self.service.list_groups().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 17. get_group_projects
    #[tool(description = "List all projects (including sub-groups) within a given group.")]
    async fn get_group_projects(
        &self,
        Parameters(params): Parameters<GetGroupProjectsParams>,
    ) -> CallToolResult {
        match self.service.get_group_projects(&params.group).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 18. list_project_files
    #[tool(description = "List all files in a project from the index.")]
    async fn list_project_files(
        &self,
        Parameters(params): Parameters<ListProjectFilesParams>,
    ) -> CallToolResult {
        match self.service.list_project_files(&params.project).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 19. list_project_repos
    #[tool(description = "List repository paths for a project.")]
    async fn list_project_repos(
        &self,
        Parameters(params): Parameters<ListProjectReposParams>,
    ) -> CallToolResult {
        match self.service.list_project_repos(&params.project).await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 20. get_project_property
    #[tool(description = "Get a per-project property value from OpenGrok.")]
    async fn get_project_property(
        &self,
        Parameters(params): Parameters<GetProjectPropertyParams>,
    ) -> CallToolResult {
        match self
            .service
            .get_project_property(&params.project, &params.name)
            .await
        {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 21. get_repo_property
    #[tool(
        description = "Get a repository property (type, branch, working, remote, parent, currentVersion, historyEnabled)."
    )]
    async fn get_repo_property(
        &self,
        Parameters(params): Parameters<GetRepoPropertyParams>,
    ) -> CallToolResult {
        match self
            .service
            .get_repo_property(&params.field, &params.repository)
            .await
        {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 22. get_suggest_config
    #[tool(description = "Get the suggester configuration (enabled fields, limits, behavior).")]
    async fn get_suggest_config(
        &self,
        Parameters(_params): Parameters<NoParams>,
    ) -> CallToolResult {
        match self.service.get_suggest_config().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 23. get_opengrok_version
    #[tool(description = "Get the OpenGrok web application version string.")]
    async fn get_opengrok_version(
        &self,
        Parameters(_params): Parameters<NoParams>,
    ) -> CallToolResult {
        match self.service.get_opengrok_version().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 24. get_index_time
    #[tool(description = "Get the time of the last index run (ISO 8601 format).")]
    async fn get_index_time(&self, Parameters(_params): Parameters<NoParams>) -> CallToolResult {
        match self.service.get_index_time().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // 25. health_check
    #[tool(description = "Check whether the OpenGrok web application is alive and responding.")]
    async fn health_check(&self, Parameters(_params): Parameters<NoParams>) -> CallToolResult {
        match self.service.health_check().await {
            Ok(text) => Self::text_result(text),
            Err(e) => Self::error_result(e.to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// ServerHandler implementation
// ---------------------------------------------------------------------------

#[tool_handler]
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
