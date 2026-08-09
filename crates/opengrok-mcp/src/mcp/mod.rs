// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! MCP server module — re-exports and tool router.

mod server;
pub mod tools;
mod tools_impl;

pub use server::OpengrokServer;

use opengrok_core::domain::OpengrokRepository;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router};

use self::tools::*;

// ---------------------------------------------------------------------------
// Tool router — thin delegators to tools_impl handlers
// ---------------------------------------------------------------------------

#[tool_router]
impl<R: OpengrokRepository + Send + Sync + 'static> OpengrokServer<R> {
    #[tool(
        description = "Full-text search in OpenGrok code index (Lucene syntax). Use for finding code by keywords, function names, strings, etc."
    )]
    async fn search_code(&self, params: Parameters<SearchCodeParams>) -> CallToolResult {
        tools_impl::search::search_code(self, params).await
    }

    #[tool(
        description = "Find where a symbol is defined (classes, functions, variables). Returns file paths and line numbers of definitions."
    )]
    async fn search_definition(
        &self,
        params: Parameters<SearchDefinitionParams>,
    ) -> CallToolResult {
        tools_impl::search::search_definition(self, params).await
    }

    #[tool(description = "Find all references/uses of a symbol across the codebase.")]
    async fn search_references(
        &self,
        params: Parameters<SearchReferencesParams>,
    ) -> CallToolResult {
        tools_impl::search::search_references(self, params).await
    }

    #[tool(
        description = "Search for files by path pattern (glob-style). E.g. 'MainActivity.java' or '*.xml'."
    )]
    async fn search_file_path(&self, params: Parameters<SearchFilePathParams>) -> CallToolResult {
        tools_impl::search::search_file_path(self, params).await
    }

    #[tool(description = "Search file history/changelog for matching entries.")]
    async fn search_history(&self, params: Parameters<SearchHistoryParams>) -> CallToolResult {
        tools_impl::search::search_history(self, params).await
    }

    #[tool(
        description = "Advanced search with full field access (full-text, definitions, references, path, history, type filter, pagination, sorting)."
    )]
    async fn advanced_search(&self, params: Parameters<AdvancedSearchParams>) -> CallToolResult {
        tools_impl::search::advanced_search(self, params).await
    }

    #[tool(
        description = "Get autocomplete suggestions for a partial query in the given project and field."
    )]
    async fn suggest(&self, params: Parameters<SuggestParams>) -> CallToolResult {
        tools_impl::content::suggest(self, params).await
    }

    #[tool(
        description = "Retrieve the raw content of a file from OpenGrok. Returns the file text."
    )]
    async fn get_file_content(&self, params: Parameters<GetFileContentParams>) -> CallToolResult {
        tools_impl::content::get_file_content(self, params).await
    }

    #[tool(
        description = "List all definitions (functions, classes, methods, variables) found within a specific file."
    )]
    async fn get_file_definitions(
        &self,
        params: Parameters<GetFileDefinitionsParams>,
    ) -> CallToolResult {
        tools_impl::content::get_file_definitions(self, params).await
    }

    #[tool(
        description = "Get the analyzer-detected genre of a file (PLAIN, XREFABLE, IMAGE, DATA, HTML)."
    )]
    async fn get_file_genre(&self, params: Parameters<GetFileGenreParams>) -> CallToolResult {
        tools_impl::content::get_file_genre(self, params).await
    }

    #[tool(description = "List the contents of a directory in the source tree.")]
    async fn list_directory(&self, params: Parameters<ListDirectoryParams>) -> CallToolResult {
        tools_impl::content::list_directory(self, params).await
    }

    #[tool(description = "List all indexed (searchable) projects in OpenGrok.")]
    async fn list_indexed_projects(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::metadata::list_indexed_projects(self, params).await
    }

    #[tool(description = "List all configured projects in OpenGrok (including non-indexed).")]
    async fn list_all_projects(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::metadata::list_all_projects(self, params).await
    }

    #[tool(description = "Get the revision history (commit log) for a file. Paginated.")]
    async fn get_history(&self, params: Parameters<GetHistoryParams>) -> CallToolResult {
        tools_impl::history::get_history(self, params).await
    }

    #[tool(
        description = "Get per-line annotation (blame/git-blame) for a file. Shows revision and author for each line."
    )]
    async fn get_annotation(&self, params: Parameters<GetAnnotationParams>) -> CallToolResult {
        tools_impl::history::get_annotation(self, params).await
    }

    #[tool(description = "List all configured project groups in OpenGrok.")]
    async fn list_groups(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::metadata::list_groups(self, params).await
    }

    #[tool(description = "List all projects (including sub-groups) within a given group.")]
    async fn get_group_projects(
        &self,
        params: Parameters<GetGroupProjectsParams>,
    ) -> CallToolResult {
        tools_impl::metadata::get_group_projects(self, params).await
    }

    #[tool(
        description = "List the contents of a directory within a project. Uses the directory listing API (not recursive file tree) for scalability."
    )]
    async fn list_project_files(
        &self,
        params: Parameters<ListProjectFilesParams>,
    ) -> CallToolResult {
        tools_impl::content::list_project_files(self, params).await
    }

    #[tool(description = "List repository paths for a project.")]
    async fn list_project_repos(
        &self,
        params: Parameters<ListProjectReposParams>,
    ) -> CallToolResult {
        tools_impl::metadata::list_project_repos(self, params).await
    }

    #[tool(description = "Get a per-project property value from OpenGrok.")]
    async fn get_project_property(
        &self,
        params: Parameters<GetProjectPropertyParams>,
    ) -> CallToolResult {
        tools_impl::metadata::get_project_property(self, params).await
    }

    #[tool(
        description = "Get a repository property (type, branch, working, remote, parent, currentVersion, historyEnabled)."
    )]
    async fn get_repo_property(&self, params: Parameters<GetRepoPropertyParams>) -> CallToolResult {
        tools_impl::metadata::get_repo_property(self, params).await
    }

    #[tool(description = "Get the suggester configuration (enabled fields, limits, behavior).")]
    async fn get_suggest_config(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::metadata::get_suggest_config(self, params).await
    }

    #[tool(description = "Get the OpenGrok web application version string.")]
    async fn get_opengrok_version(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::system::get_opengrok_version(self, params).await
    }

    #[tool(description = "Get the time of the last index run (ISO 8601 format).")]
    async fn get_index_time(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::system::get_index_time(self, params).await
    }

    #[tool(description = "Check whether the OpenGrok web application is alive and responding.")]
    async fn health_check(&self, params: Parameters<NoParams>) -> CallToolResult {
        tools_impl::system::health_check(self, params).await
    }
}
