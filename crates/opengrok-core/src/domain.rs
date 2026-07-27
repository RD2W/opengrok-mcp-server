// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Domain layer: models, errors, and the [`OpengrokRepository`] trait.
//!
//! All types are pure data structures — no I/O, no framework dependencies.
//! The [`OpengrokRepository`] trait defines the contract for accessing
//! OpenGrok's REST API; implementations live in [`crate::infrastructure`].
//!
//! # Key design decisions
//! - [`LineNumber`] uses a custom deserializer because OpenGrok returns
//!   `lineNumber` as a JSON **string** (`"42"`), not an integer.
//! - All DTO structs use `#[serde(rename_all = "camelCase")]` to match
//!   the JSON wire format.
//! - [`DomainError`] uses `#[error(transparent)] #[from]` for seamless
//!   `?` propagation from underlying crates (`reqwest`, `serde_json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, de};

// ---------------------------------------------------------------------------
// LineNumber: custom string-to-u32 deserializer
// ---------------------------------------------------------------------------

/// Wrapper around `u32` that deserializes from a JSON **string**.
///
/// OpenGrok returns `"lineNumber": "106"` — a string containing a decimal
/// integer. This type handles the coercion transparently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LineNumber(pub u32);

impl LineNumber {
    /// Returns the inner `u32` value.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl From<LineNumber> for u32 {
    fn from(value: LineNumber) -> Self {
        value.0
    }
}

impl std::fmt::Display for LineNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn deserialize_line_number<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    struct LineNumberVisitor;

    impl<'de> de::Visitor<'de> for LineNumberVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a string (or integer) containing a line number")
        }

        fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
            u32::try_from(value).map_err(de::Error::custom)
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            if value.is_empty() {
                return Ok(0); // OpenGrok returns empty lineNumber for path: searches
            }
            value.parse::<u32>().map_err(de::Error::custom)
        }
    }

    let line = deserializer.deserialize_any(LineNumberVisitor)?;
    Ok(line)
}

impl<'de> Deserialize<'de> for LineNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_line_number(deserializer).map(Self)
    }
}

// ---------------------------------------------------------------------------
// Sort order
// ---------------------------------------------------------------------------

/// Search result sort order.
///
/// Maps to the `sort` query parameter of OpenGrok's `/search` endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Sort by relevance (default).
    #[default]
    Relevancy,
    /// Sort by full file path.
    FullPath,
    /// Sort by last modification time.
    LastModTime,
}

impl SortOrder {
    /// Returns the query parameter value for this sort order.
    #[must_use]
    pub const fn as_query_value(self) -> &'static str {
        match self {
            SortOrder::Relevancy => "relevancy",
            SortOrder::FullPath => "fullpath",
            SortOrder::LastModTime => "lastmodtime",
        }
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_query_value())
    }
}

// ---------------------------------------------------------------------------
// Search domain models
// ---------------------------------------------------------------------------

/// Parameters for a search request.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchRequest {
    /// Full-text search query (Lucene syntax).
    pub full: Option<String>,
    /// Definition search.
    pub def: Option<String>,
    /// Symbol/reference search.
    pub symbol: Option<String>,
    /// File path glob search.
    pub path: Option<String>,
    /// History search.
    pub hist: Option<String>,
    /// File type filter.
    pub file_type: Option<String>,
    /// Projects to scope the search. Empty = all projects.
    pub projects: Vec<String>,
    /// Maximum number of result documents.
    pub max_results: Option<u32>,
    /// Pagination: start index.
    pub start: Option<u32>,
    /// Maximum matching lines per file (0 = all).
    pub max_hits_per_file: Option<u32>,
    /// Sort order.
    pub sort: Option<SortOrder>,
}

impl SearchRequest {
    /// Returns `true` if at least one search field is populated.
    #[must_use]
    pub fn has_query(&self) -> bool {
        self.full.is_some()
            || self.def.is_some()
            || self.symbol.is_some()
            || self.path.is_some()
            || self.hist.is_some()
    }

    /// Applies service-level defaults (capped `max_hits_per_file`,
    /// default `max_results`).
    #[must_use]
    pub fn with_defaults(mut self, cap_max_hits: u32, default_max_results: u32) -> Self {
        if let Some(mh) = self.max_hits_per_file {
            self.max_hits_per_file = Some(mh.min(cap_max_hits));
        } else {
            self.max_hits_per_file = Some(cap_max_hits);
        }
        if self.max_results.is_none() {
            self.max_results = Some(default_max_results);
        }
        self
    }
}

/// A single search hit (one matching line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// Line number within the file.
    pub line_number: LineNumber,
    /// Matched line text (may contain HTML `<b>` tags).
    pub line: String,
    /// Semantic tag (e.g. "function in pickle_file").
    pub tag: String,
}

/// Hits for a single file, keyed by file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHits {
    /// Absolute file path within the source tree.
    pub path: String,
    /// Matching lines for this file.
    pub hits: Vec<SearchHit>,
}

/// Complete search response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResults {
    /// Total number of matching documents.
    pub result_count: u32,
    /// Index of the first document in this page.
    pub start_document: u32,
    /// Index of the last document in this page.
    pub end_document: u32,
    /// Server-side processing time in milliseconds.
    pub duration_ms: u64,
    /// Hits grouped by file path (stable ordering).
    pub hits_by_file: Vec<FileHits>,
}

impl SearchResults {
    /// Returns `true` if there are more results beyond this page.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.end_document + 1 < self.result_count
    }
}

// ---------------------------------------------------------------------------
// DTOs for JSON deserialization (OpenGrok wire format)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchHitDto {
    pub line: String,
    pub line_number: LineNumber,
    #[serde(default)]
    pub tag: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResponseDto {
    pub time: u64,
    pub result_count: u32,
    #[serde(default)]
    pub start_document: u32,
    pub end_document: u32,
    #[serde(default)]
    pub results: BTreeMap<String, Vec<SearchHitDto>>,
}

impl From<SearchResponseDto> for SearchResults {
    fn from(dto: SearchResponseDto) -> Self {
        let hits_by_file: Vec<FileHits> = dto
            .results
            .into_iter()
            .map(|(path, hits)| {
                let hits = hits
                    .into_iter()
                    .map(|h| SearchHit {
                        line_number: h.line_number,
                        line: h.line,
                        tag: h.tag.unwrap_or_default(),
                    })
                    .collect();
                FileHits { path, hits }
            })
            .collect();

        SearchResults {
            result_count: dto.result_count,
            start_document: dto.start_document,
            end_document: dto.end_document,
            duration_ms: dto.time,
            hits_by_file,
        }
    }
}

// ---------------------------------------------------------------------------
// File content, genre, definitions
// ---------------------------------------------------------------------------

/// File genre as reported by OpenGrok's analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FileGenre {
    /// Plain text (searchable, xrefable).
    Plain,
    /// Cross-reference-able source code.
    Xrefable,
    /// Binary image.
    Image,
    /// Non-parseable data.
    Data,
    /// HTML content.
    Html,
}

/// Raw file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContent {
    /// Path relative to source root.
    pub path: String,
    /// Raw text bytes (decoded to string lossily).
    pub text: String,
}

/// A definition within a file (function, class, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDefinition {
    /// Kind of the definition (e.g. "function", "method", "class").
    #[allow(dead_code)]
    pub def_type: String,
    /// Full signature.
    #[allow(dead_code)]
    pub signature: String,
    /// Human-readable text.
    #[allow(dead_code)]
    pub text: String,
    /// Symbol name.
    pub symbol: String,
    /// Start line (1-based).
    #[allow(dead_code)]
    pub line_start: u32,
    /// End line (1-based).
    #[allow(dead_code)]
    pub line_end: u32,
    /// Definition line.
    #[allow(dead_code)]
    pub line: u32,
    /// Namespace if known.
    #[allow(dead_code)]
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileDefinitionDto {
    pub r#type: String,
    pub signature: String,
    pub text: String,
    pub symbol: String,
    #[serde(default)]
    pub line_start: u32,
    pub line_end: u32,
    pub line: u32,
    #[serde(default)]
    pub namespace: Option<String>,
}

impl From<FileDefinitionDto> for FileDefinition {
    fn from(dto: FileDefinitionDto) -> Self {
        Self {
            def_type: dto.r#type,
            signature: dto.signature,
            text: dto.text,
            symbol: dto.symbol,
            line_start: dto.line_start,
            line_end: dto.line_end,
            line: dto.line,
            namespace: dto.namespace,
        }
    }
}

// ---------------------------------------------------------------------------
// Directory listing
// ---------------------------------------------------------------------------

/// A single directory entry (file or subdirectory).
#[derive(Debug, Clone, PartialEq)]
pub struct DirectoryEntry {
    /// Path relative to source root.
    pub path: String,
    /// Whether this entry is a directory.
    pub is_directory: bool,
    /// Line count for files (0 for directories).
    #[allow(dead_code)]
    pub num_lines: u32,
    /// Lines of code (0 for directories).
    #[allow(dead_code)]
    pub loc: u32,
    /// Last modification date as epoch milliseconds.
    #[allow(dead_code)]
    pub date: Option<i64>,
    /// Description (human-readable, optional).
    #[allow(dead_code)]
    pub description: Option<String>,
    /// File size in bytes (None for directories).
    #[allow(dead_code)]
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntryDto {
    pub path: String,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    #[serde(default)]
    pub num_lines: u32,
    pub loc: u32,
    #[serde(default)]
    pub date: Option<i64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

impl From<DirectoryEntryDto> for DirectoryEntry {
    fn from(dto: DirectoryEntryDto) -> Self {
        Self {
            path: dto.path,
            is_directory: dto.is_directory,
            num_lines: dto.num_lines,
            loc: dto.loc,
            date: dto.date,
            description: dto.description,
            size: dto.size,
        }
    }
}

// ---------------------------------------------------------------------------
// History and annotation
// ---------------------------------------------------------------------------

/// Request parameters for `/history`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRequest {
    pub path: String,
    pub start: Option<u32>,
    pub max: Option<u32>,
    pub with_files: Option<bool>,
}

/// A single history entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// Revision hash/number.
    pub revision: String,
    /// Author name.
    pub author: String,
    /// Timestamp as epoch milliseconds.
    pub date: i64,
    /// Commit message.
    pub message: String,
    /// Tags (e.g. branch names).
    pub tags: Vec<String>,
    /// Files changed in this revision.
    pub files: Vec<String>,
}

/// History response with pagination info.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryResponse {
    pub entries: Vec<HistoryEntry>,
    pub start: u32,
    pub count: u32,
    pub total: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryEntryDto {
    pub revision: String,
    pub author: String,
    pub date: i64,
    pub message: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryResponseDto {
    pub entries: Vec<HistoryEntryDto>,
    pub start: u32,
    pub count: u32,
    pub total: u32,
}

impl From<HistoryResponseDto> for HistoryResponse {
    fn from(dto: HistoryResponseDto) -> Self {
        Self {
            entries: dto
                .entries
                .into_iter()
                .map(|e| HistoryEntry {
                    revision: e.revision,
                    author: e.author,
                    date: e.date,
                    message: e.message,
                    tags: e.tags,
                    files: e.files,
                })
                .collect(),
            start: dto.start,
            count: dto.count,
            total: dto.total,
        }
    }
}

/// A single annotation (blame) entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationEntry {
    /// Revision hash/number.
    pub revision: String,
    /// Author name.
    pub author: String,
    /// Line description.
    pub description: String,
    /// Version string (e.g. "14/15").
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AnnotationEntryDto {
    pub revision: String,
    pub author: String,
    pub description: String,
    pub version: String,
}

impl From<AnnotationEntryDto> for AnnotationEntry {
    fn from(dto: AnnotationEntryDto) -> Self {
        Self {
            revision: dto.revision,
            author: dto.author,
            description: dto.description,
            version: dto.version,
        }
    }
}

// ---------------------------------------------------------------------------
// Suggester
// ---------------------------------------------------------------------------

/// Request parameters for `/suggest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestRequest {
    pub projects: Vec<String>,
    pub field: String,
    pub caret: u32,
    pub full: Option<String>,
    pub defs: Option<String>,
    pub refs: Option<String>,
    pub path: Option<String>,
    pub hist: Option<String>,
    pub file_type: Option<String>,
}

/// A suggestion from the suggester.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub phrase: String,
    #[allow(dead_code)]
    pub projects: Vec<String>,
    pub score: Option<i32>,
}

// ---------------------------------------------------------------------------
// Suggester configuration
// ---------------------------------------------------------------------------

/// Configuration of the OpenGrok suggester.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestConfig {
    pub enabled: bool,
    pub max_results: u32,
    pub min_chars: u32,
    #[allow(dead_code)]
    pub allowed_projects: Option<Vec<String>>,
    pub max_projects: u32,
    pub allowed_fields: Vec<String>,
    pub allow_complex_queries: bool,
    pub allow_most_popular: bool,
    pub show_scores: bool,
    pub show_projects: bool,
    pub show_time: bool,
    #[allow(dead_code)]
    pub rebuild_cron_config: String,
    #[allow(dead_code)]
    pub build_termination_time: u32,
    #[allow(dead_code)]
    pub rebuild_thread_pool_size_in_ncpu_percent: u32,
    #[allow(dead_code)]
    pub search_thread_pool_size_in_ncpu_percent: u32,
}

// ---------------------------------------------------------------------------
// Domain errors
// ---------------------------------------------------------------------------

/// Unified error type for the domain layer.
///
/// Uses `#[error(transparent)] #[from]` so that `?` propagates
/// underlying errors (`reqwest::Error`, `serde_json::Error`, etc.)
/// without explicit `.map_err()` at every call site.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Search requires at least one of: full, def, symbol, path, hist.
    #[error("opengrok: search requires at least one of: full, def, symbol, path, hist")]
    EmptyQuery,

    /// An invalid project name was provided.
    #[error("opengrok: invalid project name: {0}")]
    InvalidProject(String),

    /// OpenGrok returned a non-2xx response.
    #[error("opengrok: HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },

    /// Network/transport error (DNS, TLS, timeout, connection refused).
    #[error("opengrok: network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON decode error (malformed response).
    #[error("opengrok: decode error: {0}")]
    Decode(#[from] serde_json::Error),

    /// TLS or certificate error.
    #[error("opengrok: TLS/cert error: {0}")]
    Tls(String),

    /// Cache operation failed.
    #[error(transparent)]
    Cache(#[from] CacheError),

    /// Rate limit exceeded.
    #[error(transparent)]
    RateLimit(#[from] RateLimitError),

    /// The requested operation is not yet implemented.
    #[error("opengrok: not yet implemented")]
    NotImplemented,
}

/// Cache-related errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CacheError {
    #[error("cache capacity exceeded")]
    CapacityExceeded,
}

/// Rate-limit errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded, retry after {retry_after_secs}s")]
    Exceeded { retry_after_secs: u64 },
}

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

    /// List files within a project from the index.
    async fn list_project_files(&self, project: &str) -> Result<Vec<String>, DomainError>;

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
    project_files_results: std::sync::Mutex<Vec<Result<Vec<String>, DomainError>>>,
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
            project_files_results: std::sync::Mutex::new(Vec::new()),
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
    pub fn push_project_files(&self, result: Result<Vec<String>, DomainError>) {
        self.project_files_results.lock().unwrap().push(result);
    }

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

    async fn list_project_files(&self, _project: &str) -> Result<Vec<String>, DomainError> {
        self.project_files_results
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- LineNumber deserialization -----------------------------------------

    #[test]
    fn line_number_from_string() {
        let json = r#"{"ln": "42"}"#;
        #[derive(Deserialize)]
        struct Test {
            #[allow(dead_code)]
            ln: LineNumber,
        }
        let t: Test = serde_json::from_str(json).unwrap();
        assert_eq!(t.ln.get(), 42);
    }

    #[test]
    fn line_number_from_integer() {
        let json = r#"{"ln": 7}"#;
        #[derive(Deserialize)]
        struct Test {
            #[allow(dead_code)]
            ln: LineNumber,
        }
        let t: Test = serde_json::from_str(json).unwrap();
        assert_eq!(t.ln.get(), 7);
    }

    #[test]
    fn line_number_invalid_string_returns_error() {
        let json = r#"{"ln": "not-a-number"}"#;
        #[derive(Deserialize)]
        struct Test {
            #[allow(dead_code)]
            ln: LineNumber,
        }
        let result: Result<Test, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn line_number_display() {
        let ln = LineNumber(99);
        assert_eq!(ln.to_string(), "99");
    }

    // -- SortOrder -----------------------------------------------------------

    #[test]
    fn sort_order_default_is_relevancy() {
        assert_eq!(SortOrder::default(), SortOrder::Relevancy);
    }

    #[test]
    fn sort_order_query_values() {
        assert_eq!(SortOrder::Relevancy.as_query_value(), "relevancy");
        assert_eq!(SortOrder::FullPath.as_query_value(), "fullpath");
        assert_eq!(SortOrder::LastModTime.as_query_value(), "lastmodtime");
    }

    // -- SearchRequest -------------------------------------------------------

    #[test]
    fn search_request_has_query_returns_true_when_full_set() {
        let req = SearchRequest {
            full: Some("test".into()),
            ..Default::default()
        };
        assert!(req.has_query());
    }

    #[test]
    fn search_request_has_query_returns_false_when_empty() {
        let req = SearchRequest::default();
        assert!(!req.has_query());
    }

    #[test]
    fn search_request_with_defaults_caps_max_hits() {
        let req = SearchRequest {
            max_hits_per_file: Some(50),
            ..Default::default()
        };
        let capped = req.with_defaults(10, 25);
        assert_eq!(capped.max_hits_per_file, Some(10));
    }

    #[test]
    fn search_request_with_defaults_sets_default_max_results() {
        let req = SearchRequest::default();
        let capped = req.with_defaults(10, 25);
        assert_eq!(capped.max_results, Some(25));
    }

    #[test]
    fn search_request_with_defaults_preserves_explicit_max_results() {
        let req = SearchRequest {
            max_results: Some(50),
            ..Default::default()
        };
        let capped = req.with_defaults(10, 25);
        assert_eq!(capped.max_results, Some(50));
    }

    // -- SearchResults -------------------------------------------------------

    #[test]
    fn search_results_has_more_true_when_end_plus_one_lt_count() {
        let results = SearchResults {
            result_count: 20,
            start_document: 0,
            end_document: 9,
            duration_ms: 0,
            hits_by_file: vec![],
        };
        assert!(results.has_more());
    }

    #[test]
    fn search_results_has_more_false_when_last_page() {
        let results = SearchResults {
            result_count: 10,
            start_document: 0,
            end_document: 9,
            duration_ms: 0,
            hits_by_file: vec![],
        };
        assert!(!results.has_more());
    }

    // -- SearchResponseDto → SearchResults ---------------------------------

    #[test]
    fn search_response_dto_conversion() {
        let json = r#"{
            "time": 1229,
            "resultCount": 2,
            "startDocument": 0,
            "endDocument": 1,
            "results": {
                "/src/main.rs": [
                    {"line": "fn <b>main</b>() {}", "lineNumber": "42", "tag": "function"}
                ],
                "/src/lib.rs": [
                    {"line": "pub fn init()", "lineNumber": "7", "tag": "function"}
                ]
            }
        }"#;
        let dto: SearchResponseDto = serde_json::from_str(json).unwrap();
        let results: SearchResults = dto.into();

        assert_eq!(results.result_count, 2);
        assert_eq!(results.duration_ms, 1229);
        assert_eq!(results.hits_by_file.len(), 2);
        assert!(
            results
                .hits_by_file
                .iter()
                .any(|f| f.path == "/src/main.rs")
        );
        assert!(results.hits_by_file.iter().any(|f| f.path == "/src/lib.rs"));
    }

    #[test]
    fn search_response_dto_null_tag_and_empty_line_number() {
        // Real OpenGrok response for path: search — null tag, empty lineNumber
        let json = r#"{
            "time": 90,
            "resultCount": 1,
            "startDocument": 0,
            "endDocument": 0,
            "results": {
                "/path/to/file.java": [
                    {"line": "...", "lineNumber": "", "tag": null}
                ]
            }
        }"#;
        let dto: SearchResponseDto = serde_json::from_str(json).unwrap();
        let results: SearchResults = dto.into();

        assert_eq!(results.result_count, 1);
        assert_eq!(results.hits_by_file.len(), 1);
        let hit = &results.hits_by_file[0].hits[0];
        assert_eq!(
            hit.line_number.get(),
            0,
            "empty lineNumber should parse as 0"
        );
        assert_eq!(hit.tag, "", "null tag should default to empty string");
    }

    // -- FileGenre -----------------------------------------------------------

    #[test]
    fn file_genre_deserialization() {
        let json = r#""PLAIN""#;
        let g: FileGenre = serde_json::from_str(json).unwrap();
        assert_eq!(g, FileGenre::Plain);

        let json = r#""XREFABLE""#;
        let g: FileGenre = serde_json::from_str(json).unwrap();
        assert_eq!(g, FileGenre::Xrefable);
    }

    // -- DomainError ---------------------------------------------------------

    #[test]
    fn domain_error_display() {
        let err = DomainError::EmptyQuery;
        assert!(err.to_string().contains("search requires"));
    }

    #[test]
    fn domain_error_tls_variant() {
        let err = DomainError::Tls("cert expired".into());
        assert!(err.to_string().contains("cert expired"));
    }

    // -- MockOpengrokRepository ----------------------------------------------

    fn make_hit(line_number: u32, line: &str) -> SearchHit {
        SearchHit {
            line_number: LineNumber(line_number),
            line: line.into(),
            tag: "test".into(),
        }
    }

    #[tokio::test]
    async fn mock_search_returns_pushed_result() {
        let mock = MockOpengrokRepository::new();
        let results = SearchResults {
            result_count: 1,
            start_document: 0,
            end_document: 0,
            duration_ms: 5,
            hits_by_file: vec![FileHits {
                path: "/a.rs".into(),
                hits: vec![make_hit(1, "hello")],
            }],
        };
        mock.push_ok_search(results);

        let req = SearchRequest {
            full: Some("hello".into()),
            ..Default::default()
        };
        let result = mock.search(&req).await.unwrap();
        assert_eq!(result.result_count, 1);
        assert_eq!(result.hits_by_file[0].path, "/a.rs");
        assert_eq!(mock.search_call_count(), 1);
    }

    #[tokio::test]
    async fn mock_search_tracks_last_request() {
        let mock = MockOpengrokRepository::new();
        mock.push_ok_search(SearchResults {
            result_count: 0,
            start_document: 0,
            end_document: 0,
            duration_ms: 0,
            hits_by_file: vec![],
        });

        let req = SearchRequest {
            full: Some("tracked".into()),
            projects: vec!["proj".into()],
            max_results: Some(10),
            ..Default::default()
        };
        let _ = mock.search(&req).await.unwrap();

        let last = mock.last_search_request().unwrap();
        assert_eq!(last.full, Some("tracked".into()));
        assert_eq!(last.projects, vec!["proj"]);
        assert_eq!(last.max_results, Some(10));
    }

    #[tokio::test]
    async fn mock_search_returns_error() {
        let mock = MockOpengrokRepository::new();
        mock.push_err_search(DomainError::EmptyQuery);

        let req = SearchRequest::default();
        let result = mock.search(&req).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::EmptyQuery));
    }

    #[tokio::test]
    async fn mock_search_returns_not_implemented_when_empty() {
        let mock = MockOpengrokRepository::new();
        let req = SearchRequest::default();
        let result = mock.search(&req).await;
        assert!(matches!(result, Err(DomainError::NotImplemented)));
    }
}
