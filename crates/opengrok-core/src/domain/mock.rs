// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Mock OpengrokRepository implementation for tests.

use super::*;

// ---------------------------------------------------------------------------
// Mock repository (for tests)
// ---------------------------------------------------------------------------

/// In-memory mock of [`OpengrokRepository`] for use in tests.
///
/// Scripts pre-canned responses and tracks call counts and
/// last request parameters.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MockOpengrokRepository {
    // Search scripting
    search_results: std::sync::Mutex<Vec<Result<SearchResults, DomainError>>>,
    search_call_count: std::sync::atomic::AtomicUsize,
    last_search: std::sync::RwLock<Option<SearchRequest>>,
    // Suggest
    suggestion_results: std::sync::Mutex<Vec<Result<Vec<Suggestion>, DomainError>>>,
    // File content
    file_content_results: std::sync::Mutex<Vec<Result<FileContent, DomainError>>>,
    // File definitions
    file_defs_results: std::sync::Mutex<Vec<Result<Vec<FileDefinition>, DomainError>>>,
    // File genre
    file_genre_results: std::sync::Mutex<Vec<Result<FileGenre, DomainError>>>,
    // Directory listing
    dir_list_results: std::sync::Mutex<Vec<Result<Vec<DirectoryEntry>, DomainError>>>,
    // Projects (None = not yet set → NotImplemented)
    indexed_projects_result: std::sync::Mutex<Option<Vec<String>>>,
    all_projects_result: std::sync::Mutex<Option<Vec<String>>>,
    // History
    history_results: std::sync::Mutex<Vec<Result<HistoryResponse, DomainError>>>,
    // Annotation
    annotation_results: std::sync::Mutex<Vec<Result<Vec<AnnotationEntry>, DomainError>>>,
    // New: groups, projects extra, repos, system, suggest config, index
    groups_result: std::sync::Mutex<Option<Vec<String>>>,
    group_projects_results: std::sync::Mutex<Vec<Result<Vec<String>, DomainError>>>,
    project_repos_results: std::sync::Mutex<Vec<Result<Vec<String>, DomainError>>>,
    project_property_results: std::sync::Mutex<Vec<Result<String, DomainError>>>,
    repo_property_results: std::sync::Mutex<Vec<Result<String, DomainError>>>,
    suggest_config_result: std::sync::Mutex<Option<SuggestConfig>>,
    system_ping_alive: std::sync::atomic::AtomicBool,
    system_version_result: std::sync::Mutex<Option<String>>,
    system_indextime_result: std::sync::Mutex<Option<String>>,
}

impl Default for MockOpengrokRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl MockOpengrokRepository {
    /// Creates a new mock with no pre-canned responses.
    pub fn new() -> Self {
        Self {
            search_results: std::sync::Mutex::new(Vec::new()),
            search_call_count: std::sync::atomic::AtomicUsize::new(0),
            last_search: std::sync::RwLock::new(None),
            suggestion_results: std::sync::Mutex::new(Vec::new()),
            file_content_results: std::sync::Mutex::new(Vec::new()),
            file_defs_results: std::sync::Mutex::new(Vec::new()),
            file_genre_results: std::sync::Mutex::new(Vec::new()),
            dir_list_results: std::sync::Mutex::new(Vec::new()),
            indexed_projects_result: std::sync::Mutex::new(None),
            all_projects_result: std::sync::Mutex::new(None),
            history_results: std::sync::Mutex::new(Vec::new()),
            annotation_results: std::sync::Mutex::new(Vec::new()),
            groups_result: std::sync::Mutex::new(None),
            group_projects_results: std::sync::Mutex::new(Vec::new()),
            project_repos_results: std::sync::Mutex::new(Vec::new()),
            project_property_results: std::sync::Mutex::new(Vec::new()),
            repo_property_results: std::sync::Mutex::new(Vec::new()),
            suggest_config_result: std::sync::Mutex::new(None),
            system_ping_alive: std::sync::atomic::AtomicBool::new(true),
            system_version_result: std::sync::Mutex::new(None),
            system_indextime_result: std::sync::Mutex::new(None),
        }
    }

    // -- Search scripting ---------------------------------------------------

    /// Push a canned search response (consumed in FIFO order).
    pub fn push_search(&self, result: Result<SearchResults, DomainError>) {
        self.search_results.lock().unwrap().push(result);
    }

    /// Returns the number of search calls made.
    pub fn search_call_count(&self) -> usize {
        self.search_call_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns the last [`SearchRequest`] that was passed to `search()`.
    pub fn last_search_request(&self) -> Option<SearchRequest> {
        self.last_search.read().unwrap().clone()
    }

    // -- Convenience builders -----------------------------------------------

    /// Push a successful search result with the given hits.
    pub fn push_ok_search(&self, results: SearchResults) {
        self.push_search(Ok(results));
    }

    /// Push a search error.
    pub fn push_err_search(&self, err: DomainError) {
        self.push_search(Err(err));
    }

    // -- Other methods ------------------------------------------------------

    /// Push a canned suggestion result.
    pub fn push_suggestions(&self, result: Result<Vec<Suggestion>, DomainError>) {
        self.suggestion_results.lock().unwrap().push(result);
    }

    /// Set the result for `get_file_content`.
    pub fn push_file_content(&self, result: Result<FileContent, DomainError>) {
        self.file_content_results.lock().unwrap().push(result);
    }

    /// Set the result for `get_file_definitions`.
    pub fn push_file_defs(&self, result: Result<Vec<FileDefinition>, DomainError>) {
        self.file_defs_results.lock().unwrap().push(result);
    }

    /// Set the result for `get_file_genre`.
    pub fn push_file_genre(&self, result: Result<FileGenre, DomainError>) {
        self.file_genre_results.lock().unwrap().push(result);
    }

    /// Push a directory listing result.
    pub fn push_dir_list(&self, result: Result<Vec<DirectoryEntry>, DomainError>) {
        self.dir_list_results.lock().unwrap().push(result);
    }

    /// Set the result for `list_indexed_projects`.
    pub fn set_indexed_projects(&self, projects: Vec<String>) {
        *self.indexed_projects_result.lock().unwrap() = Some(projects);
    }

    /// Set the result for `list_all_projects`.
    pub fn set_all_projects(&self, projects: Vec<String>) {
        *self.all_projects_result.lock().unwrap() = Some(projects);
    }

    /// Push a history result.
    pub fn push_history(&self, result: Result<HistoryResponse, DomainError>) {
        self.history_results.lock().unwrap().push(result);
    }

    /// Push an annotation result.
    pub fn push_annotation(&self, result: Result<Vec<AnnotationEntry>, DomainError>) {
        self.annotation_results.lock().unwrap().push(result);
    }

    /// Set the result for `list_groups`.
    pub fn set_groups(&self, groups: Vec<String>) {
        *self.groups_result.lock().unwrap() = Some(groups);
    }

    /// Push a group projects result.
    pub fn push_group_projects(&self, result: Result<Vec<String>, DomainError>) {
        self.group_projects_results.lock().unwrap().push(result);
    }

    /// Push a project files result.
    /// Push a project repos result.
    pub fn push_project_repos(&self, result: Result<Vec<String>, DomainError>) {
        self.project_repos_results.lock().unwrap().push(result);
    }

    /// Push a project property result.
    pub fn push_project_property(&self, result: Result<String, DomainError>) {
        self.project_property_results.lock().unwrap().push(result);
    }

    /// Push a repo property result.
    pub fn push_repo_property(&self, result: Result<String, DomainError>) {
        self.repo_property_results.lock().unwrap().push(result);
    }

    /// Set the result for `get_suggest_config`.
    pub fn set_suggest_config(&self, config: SuggestConfig) {
        *self.suggest_config_result.lock().unwrap() = Some(config);
    }

    /// Set whether health_check returns true or false.
    pub fn set_ping_alive(&self, alive: bool) {
        self.system_ping_alive
            .store(alive, std::sync::atomic::Ordering::Relaxed);
    }

    /// Set the result for `get_opengrok_version`.
    pub fn set_version(&self, version: String) {
        *self.system_version_result.lock().unwrap() = Some(version);
    }

    /// Set the result for `get_index_time`.
    pub fn set_index_time(&self, time: String) {
        *self.system_indextime_result.lock().unwrap() = Some(time);
    }
}

#[async_trait::async_trait]
impl OpengrokRepository for MockOpengrokRepository {
    async fn search(&self, req: &SearchRequest) -> Result<SearchResults, DomainError> {
        self.search_call_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last_search.write().unwrap() = Some(req.clone());
        self.search_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn suggest(&self, _req: &SuggestRequest) -> Result<Vec<Suggestion>, DomainError> {
        self.suggestion_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_file_content(
        &self,
        _project: &str,
        _path: &str,
    ) -> Result<FileContent, DomainError> {
        self.file_content_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_file_definitions(&self, _path: &str) -> Result<Vec<FileDefinition>, DomainError> {
        self.file_defs_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_file_genre(&self, _path: &str) -> Result<FileGenre, DomainError> {
        self.file_genre_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn list_directory(&self, _path: &str) -> Result<Vec<DirectoryEntry>, DomainError> {
        self.dir_list_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn list_indexed_projects(&self) -> Result<Vec<String>, DomainError> {
        self.indexed_projects_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn list_all_projects(&self) -> Result<Vec<String>, DomainError> {
        self.all_projects_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn get_history(&self, _req: &HistoryRequest) -> Result<HistoryResponse, DomainError> {
        self.history_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_annotation(&self, _path: &str) -> Result<Vec<AnnotationEntry>, DomainError> {
        self.annotation_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn list_groups(&self) -> Result<Vec<String>, DomainError> {
        self.groups_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn get_group_projects(&self, _group: &str) -> Result<Vec<String>, DomainError> {
        self.group_projects_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn list_project_repos(&self, _project: &str) -> Result<Vec<String>, DomainError> {
        self.project_repos_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_project_property(
        &self,
        _project: &str,
        _name: &str,
    ) -> Result<String, DomainError> {
        self.project_property_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_repo_property(
        &self,
        _field: &str,
        _repository: &str,
    ) -> Result<String, DomainError> {
        self.repo_property_results
            .lock()
            .unwrap()
            .pop()
            .unwrap_or(Err(DomainError::NotImplemented))
    }

    async fn get_suggest_config(&self) -> Result<SuggestConfig, DomainError> {
        self.suggest_config_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn get_opengrok_version(&self) -> Result<String, DomainError> {
        self.system_version_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn get_index_time(&self) -> Result<String, DomainError> {
        self.system_indextime_result
            .lock()
            .unwrap()
            .clone()
            .ok_or(DomainError::NotImplemented)
    }

    async fn health_check(&self) -> Result<bool, DomainError> {
        Ok(self
            .system_ping_alive
            .load(std::sync::atomic::Ordering::Relaxed))
    }
}
