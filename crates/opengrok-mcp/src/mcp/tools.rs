// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Tool parameter types (input schemas for MCP tools).
//!
//! Each struct defines the JSON Schema for the corresponding MCP tool.
//! Derives [`schemars::JsonSchema`] so rmcp can generate schemas
//! automatically for MCP clients.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -- Search ----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchCodeParams {
    #[schemars(description = "Full-text search query (Lucene syntax)")]
    pub query: String,
    #[schemars(description = "Project name to search in (empty = all)")]
    #[serde(default)]
    pub project: Option<String>,
    #[schemars(description = "Maximum number of result documents")]
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchDefinitionParams {
    #[schemars(description = "Symbol name to find definitions for")]
    pub symbol: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchReferencesParams {
    #[schemars(description = "Symbol name to find references for")]
    pub symbol: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchFilePathParams {
    #[schemars(description = "File path glob pattern (Lucene syntax)")]
    pub path: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchHistoryParams {
    #[schemars(description = "History search query")]
    pub hist: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AdvancedSearchParams {
    #[schemars(description = "Full-text search")]
    #[serde(default)]
    pub full: Option<String>,
    #[schemars(description = "Definition search")]
    #[serde(default)]
    pub def: Option<String>,
    #[schemars(description = "Symbol/reference search")]
    #[serde(default)]
    pub symbol: Option<String>,
    #[schemars(description = "File path glob search")]
    #[serde(default)]
    pub path: Option<String>,
    #[schemars(description = "History search")]
    #[serde(default)]
    pub hist: Option<String>,
    #[schemars(description = "File type filter")]
    #[serde(default)]
    pub file_type: Option<String>,
    #[schemars(description = "Project name")]
    #[serde(default)]
    pub project: Option<String>,
    #[schemars(description = "Maximum result documents")]
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    #[schemars(description = "Pagination start index")]
    #[serde(default)]
    pub start: Option<i64>,
    #[schemars(description = "Maximum hits per file (0 = all)")]
    #[serde(default)]
    pub max_hits_per_file: Option<u32>,
    #[schemars(description = "Sort order: relevancy, fullpath, lastmodtime")]
    #[serde(default)]
    pub sort: Option<String>,
}

// -- Suggest ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestParams {
    #[schemars(description = "Project name")]
    pub project: String,
    #[schemars(description = "Field: full, defs, refs, path, hist, type")]
    pub field: String,
    #[schemars(description = "Caret position in the partial input")]
    pub caret: u32,
    #[schemars(description = "Full-text partial input")]
    #[serde(default)]
    pub full: Option<String>,
    #[schemars(description = "Definitions partial input")]
    #[serde(default)]
    pub defs: Option<String>,
    #[schemars(description = "References partial input")]
    #[serde(default)]
    pub refs: Option<String>,
    #[schemars(description = "Path partial input")]
    #[serde(default)]
    pub path: Option<String>,
    #[schemars(description = "Type partial input")]
    #[serde(default)]
    pub file_type: Option<String>,
}

// -- File content / definitions / genre ------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFileContentParams {
    #[schemars(description = "Project name")]
    pub project: String,
    #[schemars(description = "File path relative to project root")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFileDefinitionsParams {
    #[schemars(description = "File path relative to source root (starts with /)")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetFileGenreParams {
    #[schemars(description = "File path relative to source root")]
    pub path: String,
}

// -- Directory listing -----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDirectoryParams {
    #[schemars(description = "Directory path relative to source root (starts with /)")]
    pub path: String,
}

// -- Projects --------------------------------------------------------------

/// No params needed — rmcp generates empty schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoParams {}

// -- History / Annotation --------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetHistoryParams {
    #[schemars(description = "File path relative to source root")]
    pub path: String,
    #[schemars(description = "Pagination start")]
    #[serde(default)]
    pub start: Option<u32>,
    #[schemars(description = "Maximum entries")]
    #[serde(default)]
    pub max: Option<u32>,
    #[schemars(description = "Include changed files list")]
    #[serde(default)]
    pub with_files: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAnnotationParams {
    #[schemars(description = "File path relative to source root")]
    pub path: String,
}

// -- New public endpoints ----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetGroupProjectsParams {
    #[schemars(description = "Group name")]
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProjectFilesParams {
    #[schemars(description = "Project name")]
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListProjectReposParams {
    #[schemars(description = "Project name")]
    pub project: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetProjectPropertyParams {
    #[schemars(description = "Project name")]
    pub project: String,
    #[schemars(description = "Property name")]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetRepoPropertyParams {
    #[schemars(
        description = "Property field (type, branch, working, remote, parent, currentVersion, historyEnabled)"
    )]
    pub field: String,
    #[schemars(description = "Repository path relative to source root")]
    pub repository: String,
}

// -- Defaults --------------------------------------------------------------

const fn default_max_results() -> u32 {
    25
}
