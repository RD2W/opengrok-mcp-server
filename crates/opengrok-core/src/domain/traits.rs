// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! OpengrokRepository trait.

use super::*;

// ---------------------------------------------------------------------------
// Repository trait
// ---------------------------------------------------------------------------

/// Repository abstraction for OpenGrok API calls.
///
/// Uses `#[async_trait]` to ensure returned futures are `Send`-able.
/// All methods are async and consumers are generic over `R: OpengrokRepository`.
#[async_trait::async_trait]
pub trait OpengrokRepository: Send + Sync {
    /// Full-text / fielded search.
    async fn search(&self, req: &SearchRequest) -> Result<SearchResults, DomainError>;

    /// Index-based autocomplete.
    async fn suggest(&self, req: &SuggestRequest) -> Result<Vec<Suggestion>, DomainError>;

    /// Retrieve raw file content.
    async fn get_file_content(&self, project: &str, path: &str)
    -> Result<FileContent, DomainError>;

    /// List definitions in a file (functions, classes, etc.).
    async fn get_file_definitions(&self, path: &str) -> Result<Vec<FileDefinition>, DomainError>;

    /// Get the analyzer-detected genre of a file.
    async fn get_file_genre(&self, path: &str) -> Result<FileGenre, DomainError>;

    /// List directory contents.
    async fn list_directory(&self, path: &str) -> Result<Vec<DirectoryEntry>, DomainError>;

    /// Get indexed (searchable) projects.
    async fn list_indexed_projects(&self) -> Result<Vec<String>, DomainError>;

    /// Get all configured projects.
    async fn list_all_projects(&self) -> Result<Vec<String>, DomainError>;

    /// Get file history (paginated).
    async fn get_history(&self, req: &HistoryRequest) -> Result<HistoryResponse, DomainError>;

    /// Get per-line annotation (blame) for a file.
    async fn get_annotation(&self, path: &str) -> Result<Vec<AnnotationEntry>, DomainError>;

    /// List all configured project groups.
    async fn list_groups(&self) -> Result<Vec<String>, DomainError>;

    /// Get all projects (including subgroups) within a group.
    async fn get_group_projects(&self, group: &str) -> Result<Vec<String>, DomainError>;

    /// List repository paths for a project.
    async fn list_project_repos(&self, project: &str) -> Result<Vec<String>, DomainError>;

    /// Get a per-project property value.
    async fn get_project_property(&self, project: &str, name: &str) -> Result<String, DomainError>;

    /// Get a repository property (type, branch, version, etc.).
    async fn get_repo_property(&self, field: &str, repository: &str)
    -> Result<String, DomainError>;

    /// Get suggester configuration.
    async fn get_suggest_config(&self) -> Result<SuggestConfig, DomainError>;

    /// Get OpenGrok web application version.
    async fn get_opengrok_version(&self) -> Result<String, DomainError>;

    /// Get the time of the last index run (ISO 8601).
    async fn get_index_time(&self) -> Result<String, DomainError>;

    /// Check whether the OpenGrok web application is alive.
    async fn health_check(&self) -> Result<bool, DomainError>;
}
