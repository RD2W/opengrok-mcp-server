// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Application service layer.
//!
//! [`OpengrokService`] orchestrates repository calls with caching,
//! rate-limiting, and result formatting. It is generic over the
//! repository implementation (`R: OpengrokRepository`) so that
//! tests can inject a mock.

use std::sync::Arc;
use std::time::Duration;

use crate::domain::*;
use crate::infrastructure::cache::MemoryCache;
use crate::infrastructure::format::{FormatterConfig, ResultFormatter};
use crate::infrastructure::rate_limit::TokenBucket;

// ---------------------------------------------------------------------------
// Service configuration
// ---------------------------------------------------------------------------

/// Configuration for [`OpengrokService`].
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Whether to strip HTML tags from search result lines.
    pub strip_html: bool,
    /// Cap for `max_hits_per_file` on search requests.
    pub max_hits_per_file: u32,
    /// Default `max_results` for search requests.
    pub default_max_results: u32,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            strip_html: true,
            max_hits_per_file: 10,
            default_max_results: 25,
        }
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Application service that orchestrates repository calls with
/// caching, rate-limiting, and result formatting.
#[derive(Debug)]
pub struct OpengrokService<R: OpengrokRepository> {
    repo: Arc<R>,
    cache: Option<MemoryCache<String, String>>,
    rate_limiter: Option<TokenBucket>,
    formatter: ResultFormatter,
    config: ServiceConfig,
}

// Manual Clone (avoiding R: Clone bound — Arc<R> is always Clone)
impl<R: OpengrokRepository> Clone for OpengrokService<R> {
    fn clone(&self) -> Self {
        Self {
            repo: self.repo.clone(),
            cache: self.cache.clone(),
            rate_limiter: self.rate_limiter.clone(),
            formatter: self.formatter.clone(),
            config: self.config.clone(),
        }
    }
}

impl<R: OpengrokRepository> OpengrokService<R> {
    /// Creates a new service wrapping the given repository.
    #[must_use]
    pub fn new(repo: R, config: ServiceConfig) -> Self {
        Self {
            repo: Arc::new(repo),
            cache: None,
            rate_limiter: None,
            formatter: ResultFormatter::new(FormatterConfig {
                strip_html: config.strip_html,
                ..Default::default()
            }),
            config,
        }
    }

    /// Creates a new service from an existing `Arc<R>`.
    #[must_use]
    pub fn from_arc(repo: Arc<R>, config: ServiceConfig) -> Self {
        Self {
            repo,
            cache: None,
            rate_limiter: None,
            formatter: ResultFormatter::new(FormatterConfig {
                strip_html: config.strip_html,
                ..Default::default()
            }),
            config,
        }
    }

    /// Returns the repository reference (for testing).
    #[must_use]
    pub fn repo(&self) -> &R {
        &self.repo
    }

    /// Enables caching with the given TTL and maximum entry count.
    #[must_use]
    pub fn with_cache(mut self, ttl: Duration, max_entries: usize) -> Self {
        self.cache = Some(MemoryCache::new(ttl, max_entries));
        self
    }

    /// Enables rate limiting with the given sustained rate and burst.
    #[must_use]
    pub fn with_rate_limit(mut self, requests_per_second: u32, burst: u32) -> Self {
        self.rate_limiter = Some(TokenBucket::new(requests_per_second, burst));
        self
    }

    // -- Search -------------------------------------------------------------

    /// Performs a search, with optional caching and rate-limiting.
    pub async fn search(&self, req: SearchRequest) -> Result<String, DomainError> {
        if !req.has_query() {
            return Err(DomainError::EmptyQuery);
        }

        let req = req.with_defaults(
            self.config.max_hits_per_file,
            self.config.default_max_results,
        );

        // Cache key: serialized request
        let cache_key = if self.cache.is_some() {
            Some(self.search_cache_key(&req))
        } else {
            None
        };

        // Check cache
        if let Some(ref cache) = self.cache
            && let Some(ref key) = cache_key
            && let Some(cached) = cache.get(key)
        {
            return Ok(cached);
        }

        // Rate limit
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }

        // Execute
        let results = self.repo.search(&req).await?;
        let formatted = self.formatter.format_search(&results);

        // Store in cache
        if let Some(ref cache) = self.cache
            && let Some(ref key) = cache_key
        {
            cache.insert(key.clone(), formatted.clone());
        }

        Ok(formatted)
    }

    /// Performs autocomplete suggestions.
    pub async fn suggest(&self, req: SuggestRequest) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let suggestions = self.repo.suggest(&req).await?;
        Ok(self.formatter.format_suggestions(&suggestions))
    }

    /// Retrieves raw file content (not cached — files are large).
    pub async fn get_file_content(
        &self,
        project: &str,
        path: &str,
    ) -> Result<FileContent, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        self.repo.get_file_content(project, path).await
    }

    /// Retrieves definitions within a file.
    pub async fn get_file_definitions(&self, path: &str) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let defs = self.repo.get_file_definitions(path).await?;
        Ok(self.formatter.format_definitions(&defs))
    }

    /// Retrieves file genre.
    pub async fn get_file_genre(&self, path: &str) -> Result<FileGenre, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        self.repo.get_file_genre(path).await
    }

    /// Lists directory contents.
    pub async fn list_directory(&self, path: &str) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let entries = self.repo.list_directory(path).await?;
        Ok(self.formatter.format_directory(&entries))
    }

    /// Lists indexed projects.
    pub async fn list_indexed_projects(&self) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let projects = self.repo.list_indexed_projects().await?;
        Ok(self.formatter.format_projects(&projects))
    }

    /// Lists all configured projects.
    pub async fn list_all_projects(&self) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let projects = self.repo.list_all_projects().await?;
        Ok(self.formatter.format_projects(&projects))
    }

    /// Retrieves file history.
    pub async fn get_history(&self, req: HistoryRequest) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let history = self.repo.get_history(&req).await?;
        Ok(self.formatter.format_history(&history))
    }

    /// Retrieves file annotation (blame).
    pub async fn get_annotation(&self, path: &str) -> Result<String, DomainError> {
        if let Some(ref rl) = self.rate_limiter {
            rl.acquire().await?;
        }
        let entries = self.repo.get_annotation(path).await?;
        Ok(self.formatter.format_annotation(&entries))
    }

    // -- Helpers ------------------------------------------------------------

    fn search_cache_key(&self, req: &SearchRequest) -> String {
        format!(
            "search|f={:?}|d={:?}|s={:?}|p={:?}|h={:?}|t={:?}|pr={:?}|mr={:?}|srt={:?}|st={:?}|mh={:?}",
            req.full,
            req.def,
            req.symbol,
            req.path,
            req.hist,
            req.file_type,
            req.projects,
            req.max_results,
            req.sort,
            req.start,
            req.max_hits_per_file
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> (
        OpengrokService<MockOpengrokRepository>,
        Arc<MockOpengrokRepository>,
    ) {
        let repo = Arc::new(MockOpengrokRepository::new());
        let service = OpengrokService::from_arc(repo.clone(), ServiceConfig::default());
        (service, repo)
    }

    fn make_hit(line_number: u32, line: &str) -> SearchHit {
        SearchHit {
            line_number: LineNumber(line_number),
            line: line.into(),
            tag: "test".into(),
        }
    }

    // -- Search -------------------------------------------------------------

    #[tokio::test]
    async fn search_returns_formatted_results() {
        let (svc, repo) = test_service();
        repo.push_ok_search(SearchResults {
            result_count: 1,
            start_document: 0,
            end_document: 0,
            duration_ms: 5,
            hits_by_file: vec![FileHits {
                path: "/test.rs".into(),
                hits: vec![make_hit(1, "hello world")],
            }],
        });

        let req = SearchRequest {
            full: Some("hello".into()),
            ..Default::default()
        };
        let result = svc.search(req).await.unwrap();
        assert!(result.contains("Found 1 result(s)"));
        assert!(result.contains("/test.rs"));
        assert!(result.contains("hello world"));
    }

    #[tokio::test]
    async fn search_empty_query_returns_error() {
        let (svc, _repo) = test_service();
        let result = svc.search(SearchRequest::default()).await;
        assert!(matches!(result, Err(DomainError::EmptyQuery)));
    }

    #[tokio::test]
    async fn search_caches_results() {
        let (svc, repo) = test_service();
        let svc = svc.with_cache(Duration::from_secs(60), 100);

        repo.push_ok_search(SearchResults {
            result_count: 1,
            start_document: 0,
            end_document: 0,
            duration_ms: 5,
            hits_by_file: vec![FileHits {
                path: "/a.rs".into(),
                hits: vec![make_hit(1, "cached")],
            }],
        });

        let req = SearchRequest {
            full: Some("cached".into()),
            ..Default::default()
        };

        // First call: hits the repo
        let r1 = svc.search(req.clone()).await.unwrap();
        assert!(r1.contains("cached"));
        assert_eq!(repo.search_call_count(), 1);

        // Second call: should hit cache (no additional repo calls)
        let r2 = svc.search(req).await.unwrap();
        assert!(r2.contains("cached"));
        assert_eq!(repo.search_call_count(), 1, "should have used cache");
    }

    // -- File content -------------------------------------------------------

    #[tokio::test]
    async fn get_file_content_returns_raw_text() {
        let (svc, repo) = test_service();
        repo.push_file_content(Ok(FileContent {
            path: "/proj/main.rs".into(),
            text: "fn main() {}".into(),
        }));

        let result = svc.get_file_content("proj", "main.rs").await.unwrap();
        assert_eq!(result.text, "fn main() {}");
    }

    // -- Definitions --------------------------------------------------------

    #[tokio::test]
    async fn get_file_definitions_returns_formatted() {
        let (svc, repo) = test_service();
        repo.push_file_defs(Ok(vec![FileDefinition {
            def_type: "function".into(),
            signature: "fn foo()".into(),
            text: "pub fn foo()".into(),
            symbol: "foo".into(),
            line_start: 1,
            line_end: 3,
            line: 1,
            namespace: None,
        }]));

        let result = svc.get_file_definitions("/proj/main.rs").await.unwrap();
        assert!(result.contains("foo"));
        assert!(result.contains("fn foo()"));
    }

    // -- Projects -----------------------------------------------------------

    #[tokio::test]
    async fn list_indexed_projects_returns_formatted() {
        let (svc, repo) = test_service();
        repo.set_indexed_projects(vec!["p1".into(), "p2".into()]);

        let result = svc.list_indexed_projects().await.unwrap();
        assert!(result.contains("Found 2 project(s)"));
        assert!(result.contains("p1"));
        assert!(result.contains("p2"));
    }

    // -- History ------------------------------------------------------------

    #[tokio::test]
    async fn get_history_returns_formatted() {
        let (svc, repo) = test_service();
        repo.push_history(Ok(HistoryResponse {
            entries: vec![HistoryEntry {
                revision: "abc".into(),
                author: "dev".into(),
                date: 0,
                message: "init".into(),
                tags: vec![],
                files: vec![],
            }],
            start: 0,
            count: 1,
            total: 1,
        }));

        let req = HistoryRequest {
            path: "/main.rs".into(),
            start: None,
            max: None,
            with_files: None,
        };
        let result = svc.get_history(req).await.unwrap();
        assert!(result.contains("abc"));
        assert!(result.contains("dev"));
    }

    // -- Annotation ---------------------------------------------------------

    #[tokio::test]
    async fn get_annotation_returns_formatted() {
        let (svc, repo) = test_service();
        repo.push_annotation(Ok(vec![AnnotationEntry {
            revision: "def".into(),
            author: "dev2".into(),
            description: "fix".into(),
            version: "1".into(),
        }]));

        let result = svc.get_annotation("/main.rs").await.unwrap();
        assert!(result.contains("def"));
        assert!(result.contains("dev2"));
    }

    // -- Rate limiting ------------------------------------------------------

    #[tokio::test]
    async fn rate_limiter_allows_within_limit() {
        let (svc, repo) = test_service();
        let svc = svc.with_rate_limit(1000, 500);

        repo.push_ok_search(SearchResults {
            result_count: 0,
            start_document: 0,
            end_document: 0,
            duration_ms: 0,
            hits_by_file: vec![],
        });

        let req = SearchRequest {
            full: Some("x".into()),
            ..Default::default()
        };
        let result = svc.search(req).await;
        assert!(result.is_ok());
    }
}
