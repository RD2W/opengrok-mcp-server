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

/// Wrapper DTO for the `/suggest` response (object with `suggestions` key).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SuggestResponseDto {
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
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

pub mod error;
pub mod mock;
pub mod traits;

pub use error::*;
pub use mock::MockOpengrokRepository;
pub use traits::*;
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
