// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! OpenGrok REST API HTTP client.
//!
//! Implements [`OpengrokRepository`] using `reqwest` with configurable
//! authentication (Bearer token or HTTP Basic Auth) and TLS settings.
//!
//! # Authentication priority
//! When both a username and a token are configured, Basic auth wins
//! (matching the Go reference implementation's behavior). The selected
//! mode is logged at `info` level.

use std::io;
use std::time::Duration;

use reqwest::Client;

use crate::domain::*;

use super::tls::{self, TlsConfig};

const API_PREFIX: &str = "/api/v1";
const API_SEARCH: &str = "search";
const API_SUGGEST: &str = "suggest";
const API_FILE_CONTENT: &str = "file/content";
const API_FILE_DEFS: &str = "file/defs";
const API_FILE_GENRE: &str = "file/genre";
const API_LIST: &str = "list";
const API_PROJECTS_INDEXED: &str = "projects/indexed";
const API_PROJECTS: &str = "projects";
const API_HISTORY: &str = "history";
const API_ANNOTATION: &str = "annotation";

const HEADER_ACCEPT: &str = "Accept";
const HEADER_OCTET_STREAM: &str = "application/octet-stream";

const MAX_BODY_TRUNCATION: usize = 500;

// ---------------------------------------------------------------------------
// Auth configuration
// ---------------------------------------------------------------------------

/// Authentication mode for connecting to OpenGrok.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    /// No authentication.
    None,
    /// Bearer token authentication.
    Bearer(String),
    /// HTTP Basic authentication.
    Basic { username: String, password: String },
}

// ---------------------------------------------------------------------------
// Client configuration
// ---------------------------------------------------------------------------

/// Configuration for [`OpengrokClient`].
#[derive(Debug, Clone)]
pub struct OpengrokClientConfig {
    /// Base URL of the OpenGrok instance (e.g. `https://opengrok.example.com`).
    pub base_url: String,
    /// Authentication mode (Bearer, Basic, or None).
    pub auth: AuthMode,
    /// HTTP request timeout.
    pub timeout: Duration,
    /// TLS configuration.
    pub tls: TlsConfig,
}

impl OpengrokClientConfig {
    /// Strips trailing `/` from the base URL for consistent path joining.
    #[must_use]
    pub fn normalized_base_url(&self) -> &str {
        self.base_url.trim_end_matches('/')
    }
}

// ---------------------------------------------------------------------------
// OpenGrok HTTP client
// ---------------------------------------------------------------------------

/// HTTP client for the OpenGrok REST API.
///
/// Constructed via [`OpengrokClient::new`] and implements
/// [`OpengrokRepository`].
#[derive(Debug, Clone)]
pub struct OpengrokClient {
    http: Client,
    config: OpengrokClientConfig,
}

impl OpengrokClient {
    /// Creates a new client with the given configuration.
    ///
    /// # Errors
    /// Returns [`DomainError::Tls`] if the TLS connector cannot be built.
    pub fn new(config: OpengrokClientConfig) -> Result<Self, DomainError> {
        let tls_config = tls::build_tls_connector(&config.tls)?;

        let http = Client::builder()
            .timeout(config.timeout)
            .use_preconfigured_tls(tls_config)
            .build()
            .map_err(|e| DomainError::Tls(format!("failed to build HTTP client: {e}")))?;

        tracing::info!(
            base_url = %config.normalized_base_url(),
            auth_mode = ?config.auth,
            verify_ssl = config.tls.verify_ssl,
            "OpenGrok client initialized"
        );

        Ok(Self { http, config })
    }

    // -- Internal helpers ---------------------------------------------------

    /// Apply configured authentication to a request builder.
    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth {
            AuthMode::None => req,
            AuthMode::Bearer(token) => req.bearer_auth(token),
            AuthMode::Basic { username, password } => req.basic_auth(username, Some(password)),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{API_PREFIX}/{path}", self.config.normalized_base_url())
    }

    /// Sends a GET request with optional query parameters.
    async fn get(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<reqwest::Response, DomainError> {
        let base = self.api_url(path);
        let url = if query.is_empty() {
            base
        } else {
            let qs: String = query
                .iter()
                .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{base}?{qs}")
        };

        let mut req = self.http.get(&url);
        req = self.apply_auth(req);

        let response = req.send().await?;
        let response = ensure_success(response).await?;

        Ok(response)
    }

    /// Sends a GET request and deserializes the JSON response.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, DomainError> {
        let response = self.get(path, query).await?;
        let body = response.text().await.map_err(|e| {
            DomainError::Decode(serde_json::Error::io(io::Error::other(format!(
                "failed to read response body: {e}"
            ))))
        })?;
        let value: T = serde_json::from_str(&body).map_err(|e| {
            let truncated: String = body.chars().take(MAX_BODY_TRUNCATION).collect();
            DomainError::Decode(serde_json::Error::io(io::Error::other(format!(
                "JSON parse error: {e}\nRaw body ({MAX_BODY_TRUNCATION} chars): {truncated}"
            ))))
        })?;
        Ok(value)
    }

    /// Build query parameters from a [`SearchRequest`].
    fn search_query_params(req: &SearchRequest) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();

        if let Some(ref v) = req.full {
            params.push(("full", v.clone()));
        }
        if let Some(ref v) = req.def {
            params.push(("def", v.clone()));
        }
        if let Some(ref v) = req.symbol {
            params.push(("symbol", v.clone()));
        }
        if let Some(ref v) = req.path {
            params.push(("path", v.clone()));
        }
        if let Some(ref v) = req.hist {
            params.push(("hist", v.clone()));
        }
        if let Some(ref v) = req.file_type {
            params.push(("type", v.clone()));
        }
        for p in &req.projects {
            params.push(("projects", p.clone()));
        }
        if let Some(v) = req.max_results {
            params.push(("maxresults", v.to_string()));
        }
        if let Some(v) = req.start {
            params.push(("start", v.to_string()));
        }
        if let Some(v) = req.max_hits_per_file {
            params.push(("maxhitsperfile", v.to_string()));
        }
        if let Some(s) = &req.sort {
            params.push(("sort", s.to_string()));
        }

        params
    }

    /// Build the `path` query parameter by combining project and file path.
    fn build_source_root_path(project: &str, path: &str) -> String {
        let trimmed = path.trim_start_matches('/');
        if path.is_empty() {
            format!("/{project}")
        } else if path.starts_with(&format!("/{project}")) {
            path.to_string()
        } else {
            format!("/{project}/{trimmed}")
        }
    }
}

// ---------------------------------------------------------------------------
// OpengrokRepository implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl OpengrokRepository for OpengrokClient {
    async fn search(&self, req: &SearchRequest) -> Result<SearchResults, DomainError> {
        if !req.has_query() {
            return Err(DomainError::EmptyQuery);
        }

        let params = Self::search_query_params(req);
        let dto: SearchResponseDto = self.get_json(API_SEARCH, &param_refs(&params)).await?;
        Ok(dto.into())
    }

    async fn suggest(&self, req: &SuggestRequest) -> Result<Vec<Suggestion>, DomainError> {
        let mut params: Vec<(&str, String)> = Vec::new();

        for p in &req.projects {
            params.push(("projects", p.clone()));
        }
        params.push(("field", req.field.clone()));
        params.push(("caret", req.caret.to_string()));

        if let Some(ref v) = req.full {
            params.push(("full", v.clone()));
        }
        if let Some(ref v) = req.defs {
            params.push(("defs", v.clone()));
        }
        if let Some(ref v) = req.refs {
            params.push(("refs", v.clone()));
        }
        if let Some(ref v) = req.path {
            params.push(("path", v.clone()));
        }
        if let Some(ref v) = req.hist {
            params.push(("hist", v.clone()));
        }
        if let Some(ref v) = req.file_type {
            params.push(("type", v.clone()));
        }

        let suggestions: Vec<Suggestion> = self.get_json(API_SUGGEST, &param_refs(&params)).await?;
        Ok(suggestions)
    }

    async fn get_file_content(
        &self,
        project: &str,
        path: &str,
    ) -> Result<FileContent, DomainError> {
        let source_path = Self::build_source_root_path(project, path);
        let url = format!(
            "{}?path={}",
            self.api_url(API_FILE_CONTENT),
            percent_encode(&source_path)
        );

        let req = self
            .http
            .get(&url)
            // Bypass genre check by accepting octet-stream
            .header(HEADER_ACCEPT, HEADER_OCTET_STREAM);
        let req = self.apply_auth(req);

        let response = req.send().await?;
        let response = ensure_success(response).await?;

        let text = response.text().await?;

        Ok(FileContent {
            path: source_path,
            text,
        })
    }

    async fn get_file_definitions(&self, path: &str) -> Result<Vec<FileDefinition>, DomainError> {
        let query: [(&str, &str); 1] = [("path", path)];
        let dtos: Vec<FileDefinitionDto> = self.get_json(API_FILE_DEFS, &query).await?;
        Ok(dtos.into_iter().map(Into::into).collect())
    }

    async fn get_file_genre(&self, path: &str) -> Result<FileGenre, DomainError> {
        let query: [(&str, &str); 1] = [("path", path)];
        let raw: String = self.get(API_FILE_GENRE, &query).await?.text().await?;
        let raw_trimmed = raw.trim().replace('"', "");
        let genre: FileGenre = serde_json::from_str(&format!("\"{raw_trimmed}\""))?;
        Ok(genre)
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<DirectoryEntry>, DomainError> {
        let query: [(&str, &str); 1] = [("path", path)];
        let dtos: Vec<DirectoryEntryDto> = self.get_json(API_LIST, &query).await?;
        Ok(dtos.into_iter().map(Into::into).collect())
    }

    async fn list_indexed_projects(&self) -> Result<Vec<String>, DomainError> {
        self.get_json(API_PROJECTS_INDEXED, &[]).await
    }

    async fn list_all_projects(&self) -> Result<Vec<String>, DomainError> {
        self.get_json(API_PROJECTS, &[]).await
    }

    async fn get_history(&self, req: &HistoryRequest) -> Result<HistoryResponse, DomainError> {
        let mut params: Vec<(&str, String)> = vec![("path", req.path.clone())];
        if let Some(v) = req.start {
            params.push(("start", v.to_string()));
        }
        if let Some(v) = req.max {
            params.push(("max", v.to_string()));
        }
        if let Some(v) = req.with_files {
            params.push(("withFiles", v.to_string()));
        }
        let dto: HistoryResponseDto = self.get_json(API_HISTORY, &param_refs(&params)).await?;
        Ok(dto.into())
    }

    async fn get_annotation(&self, path: &str) -> Result<Vec<AnnotationEntry>, DomainError> {
        let query: [(&str, &str); 1] = [("path", path)];
        let dtos: Vec<AnnotationEntryDto> = self.get_json(API_ANNOTATION, &query).await?;
        Ok(dtos.into_iter().map(Into::into).collect())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `Vec<(&str, String)>` into a slice of `(&str, &str)` for reqwest.
fn param_refs<'a>(params: &'a [(&'a str, String)]) -> Vec<(&'a str, &'a str)> {
    params.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

/// Minimal percent-encoding for URL query parameters.
fn percent_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' | ':' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

/// Check an HTTP response for a non-2xx status, consuming the body on error.
async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, DomainError> {
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(DomainError::HttpStatus { status, body });
    }
    Ok(response)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    // ------------------------------------------------------------------
    // Helper: spawn a tiny HTTP server for loopback testing
    // ------------------------------------------------------------------

    /// Build the full request string for loopback tests.
    async fn read_request(stream: &mut TcpStream) -> (String, Vec<String>) {
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let lines: Vec<&str> = request.lines().collect();
        let first = lines.first().map_or("", |l| *l).to_string();
        let headers: Vec<String> = lines
            .iter()
            .skip(1)
            .take_while(|l| !l.is_empty())
            .map(|&l| l.to_string())
            .collect();
        (first, headers)
    }

    /// Build a minimal client pointing at a loopback URL.
    fn test_client(port: u16) -> OpengrokClient {
        let config = OpengrokClientConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            auth: AuthMode::None,
            timeout: Duration::from_secs(5),
            tls: TlsConfig {
                verify_ssl: true,
                ..Default::default()
            },
        };
        // For loopback HTTP tests we bypass TLS entirely
        let http = Client::builder().timeout(config.timeout).build().unwrap();
        OpengrokClient { http, config }
    }

    fn bearer_client(port: u16, token: &str) -> OpengrokClient {
        let config = OpengrokClientConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            auth: AuthMode::Bearer(token.to_string()),
            timeout: Duration::from_secs(5),
            tls: TlsConfig::default(),
        };
        let http = Client::builder().timeout(config.timeout).build().unwrap();
        OpengrokClient { http, config }
    }

    fn basic_auth_client(port: u16, user: &str, pass: &str) -> OpengrokClient {
        let config = OpengrokClientConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            auth: AuthMode::Basic {
                username: user.to_string(),
                password: pass.to_string(),
            },
            timeout: Duration::from_secs(5),
            tls: TlsConfig::default(),
        };
        let http = Client::builder().timeout(config.timeout).build().unwrap();
        OpengrokClient { http, config }
    }

    // ------------------------------------------------------------------
    // build_source_root_path
    // ------------------------------------------------------------------

    #[test]
    fn source_root_path_with_empty_path() {
        let result = OpengrokClient::build_source_root_path("proj", "");
        assert_eq!(result, "/proj");
    }

    #[test]
    fn source_root_path_with_relative_path() {
        let result = OpengrokClient::build_source_root_path("proj", "src/main.rs");
        assert_eq!(result, "/proj/src/main.rs");
    }

    #[test]
    fn source_root_path_with_leading_slash() {
        let result = OpengrokClient::build_source_root_path("proj", "/src/main.rs");
        assert_eq!(result, "/proj/src/main.rs");
    }

    #[test]
    fn source_root_path_already_prefixed() {
        let result = OpengrokClient::build_source_root_path("proj", "/proj/src/main.rs");
        assert_eq!(result, "/proj/src/main.rs");
    }

    // ------------------------------------------------------------------
    // search_query_params
    // ------------------------------------------------------------------

    #[test]
    fn search_query_params_full_query() {
        let req = SearchRequest {
            full: Some("test".into()),
            projects: vec!["p1".into(), "p2".into()],
            max_results: Some(50),
            sort: Some(SortOrder::FullPath),
            ..Default::default()
        };
        let params = OpengrokClient::search_query_params(&req);
        assert!(params.iter().any(|(k, v)| *k == "full" && v == "test"));
        // projects appear multiple times
        let proj_params: Vec<_> = params.iter().filter(|(k, _)| *k == "projects").collect();
        assert_eq!(proj_params.len(), 2);
        assert_eq!(proj_params[0].1, "p1");
        assert_eq!(proj_params[1].1, "p2");
        assert!(params.iter().any(|(k, v)| *k == "maxresults" && v == "50"));
        assert!(params.iter().any(|(k, v)| *k == "sort" && v == "fullpath"));
    }

    #[test]
    fn search_query_params_all_fields() {
        let req = SearchRequest {
            full: Some("f".into()),
            def: Some("d".into()),
            symbol: Some("s".into()),
            path: Some("p".into()),
            hist: Some("h".into()),
            file_type: Some("t".into()),
            start: Some(10),
            max_hits_per_file: Some(5),
            ..Default::default()
        };
        let params = OpengrokClient::search_query_params(&req);
        assert!(params.iter().any(|(k, v)| *k == "full" && v == "f"));
        assert!(params.iter().any(|(k, v)| *k == "def" && v == "d"));
        assert!(params.iter().any(|(k, v)| *k == "symbol" && v == "s"));
        assert!(params.iter().any(|(k, v)| *k == "path" && v == "p"));
        assert!(params.iter().any(|(k, v)| *k == "hist" && v == "h"));
        assert!(params.iter().any(|(k, v)| *k == "type" && v == "t"));
        assert!(params.iter().any(|(k, v)| *k == "start" && v == "10"));
        assert!(
            params
                .iter()
                .any(|(k, v)| *k == "maxhitsperfile" && v == "5")
        );
    }

    #[test]
    fn search_query_params_empty_request() {
        let req = SearchRequest::default();
        let params = OpengrokClient::search_query_params(&req);
        assert!(params.is_empty());
    }

    // ------------------------------------------------------------------
    // HTTP loopback tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn search_hits_api_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (first, _headers) = read_request(&mut stream).await;

            assert!(
                first.contains("GET /api/v1/search"),
                "unexpected request: {first}"
            );
            assert!(
                first.contains("full=test_query"),
                "expected full=test_query in: {first}"
            );

            let body =
                r#"{"time":5,"resultCount":1,"startDocument":0,"endDocument":0,"results":{}}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let req = SearchRequest {
            full: Some("test_query".into()),
            ..Default::default()
        };
        let result = client.search(&req).await.unwrap();
        assert_eq!(result.result_count, 1);
        assert_eq!(result.duration_ms, 5);
        assert!(result.hits_by_file.is_empty());

        server.await.unwrap();
    }

    #[tokio::test]
    async fn search_empty_query_errors_before_http() {
        let client = test_client(0); // port doesn't matter
        let req = SearchRequest::default();
        let result = client.search(&req).await;
        assert!(matches!(result, Err(DomainError::EmptyQuery)));
    }

    #[tokio::test]
    async fn search_non_200_returns_http_status_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 21\r\n\r\ninternal server error";
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let req = SearchRequest {
            full: Some("x".into()),
            ..Default::default()
        };
        let result = client.search(&req).await;
        assert!(matches!(
            result,
            Err(DomainError::HttpStatus { status: 500, .. })
        ));
    }

    #[tokio::test]
    async fn search_with_bearer_auth_sends_header() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, headers) = read_request(&mut stream).await;
            let auth_header = headers
                .iter()
                .find(|h| h.to_lowercase().starts_with("authorization:"))
                .expect("no Authorization header found");

            assert!(
                auth_header.contains("Bearer"),
                "expected Bearer, got: {auth_header}"
            );
            assert!(
                auth_header.contains("my-secret-token"),
                "expected my-secret-token in: {auth_header}"
            );

            let body =
                r#"{"time":0,"resultCount":0,"startDocument":0,"endDocument":0,"results":{}}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = bearer_client(addr.port(), "my-secret-token");
        let req = SearchRequest {
            full: Some("q".into()),
            ..Default::default()
        };
        let _ = client.search(&req).await.unwrap();
    }

    #[tokio::test]
    async fn search_with_basic_auth_sends_header() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, headers) = read_request(&mut stream).await;
            let auth_header = headers
                .iter()
                .find(|h| h.to_lowercase().starts_with("authorization:"))
                .expect("no Authorization header found");
            assert!(
                auth_header.to_lowercase().contains("basic"),
                "expected Basic, got: {auth_header}"
            );

            let body =
                r#"{"time":0,"resultCount":0,"startDocument":0,"endDocument":0,"results":{}}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = basic_auth_client(addr.port(), "user", "pass");
        let req = SearchRequest {
            full: Some("q".into()),
            ..Default::default()
        };
        let _ = client.search(&req).await.unwrap();
    }

    #[tokio::test]
    async fn file_content_sends_accept_octet_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, headers) = read_request(&mut stream).await;

            let accept = headers
                .iter()
                .find(|h| h.to_lowercase().starts_with("accept:"))
                .map(|h| h.to_lowercase())
                .unwrap_or_default();
            assert!(
                accept.contains("application/octet-stream"),
                "expected application/octet-stream, got: {accept}"
            );

            let body = "fn main() {}";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let result = client
            .get_file_content("myproj", "src/main.rs")
            .await
            .unwrap();
        assert_eq!(result.text, "fn main() {}");
        assert!(result.path.contains("myproj"));
    }

    #[tokio::test]
    async fn file_content_non_200_returns_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nnot found";
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let result = client.get_file_content("proj", "missing.rs").await;
        assert!(matches!(
            result,
            Err(DomainError::HttpStatus { status: 404, .. })
        ));
    }

    #[tokio::test]
    async fn list_indexed_projects_returns_array() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, _headers) = read_request(&mut stream).await;
            let body = r#"["proj-a","proj-b","proj-c"]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let projects = client.list_indexed_projects().await.unwrap();
        assert_eq!(projects, vec!["proj-a", "proj-b", "proj-c"]);
    }

    #[tokio::test]
    async fn list_all_projects_returns_array() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, _headers) = read_request(&mut stream).await;
            let body = r#"["all-proj"]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let projects = client.list_all_projects().await.unwrap();
        assert_eq!(projects, vec!["all-proj"]);
    }

    #[tokio::test]
    async fn get_history_sends_query_params() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (first, _headers) = read_request(&mut stream).await;
            assert!(first.contains("path=/src/main.rs"));
            assert!(first.contains("start=0"));
            assert!(first.contains("max=10"));

            let body = r#"{"entries":[],"start":0,"count":0,"total":0}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let req = HistoryRequest {
            path: "/src/main.rs".into(),
            start: Some(0),
            max: Some(10),
            with_files: None,
        };
        let result = client.get_history(&req).await.unwrap();
        assert_eq!(result.total, 0);
        assert!(result.entries.is_empty());
    }

    #[tokio::test]
    async fn get_annotation_returns_entries() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (_first, _headers) = read_request(&mut stream).await;
            let body = r#"[
                {"revision": "abc123", "author": "dev", "description": "fix bug", "version": "1/10"}
            ]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let entries = client.get_annotation("/src/lib.rs").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].revision, "abc123");
        assert_eq!(entries[0].author, "dev");
    }

    #[tokio::test]
    async fn suggest_sends_query_params() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (first, _headers) = read_request(&mut stream).await;
            assert!(first.contains("projects=myproj"));
            assert!(first.contains("field=defs"));
            assert!(first.contains("caret=5"));
            assert!(first.contains("full=fn"));

            let body = r#"[{"phrase":"func","projects":[],"score":100}]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });

        let client = test_client(addr.port());
        let req = SuggestRequest {
            projects: vec!["myproj".into()],
            field: "defs".into(),
            caret: 5,
            full: Some("fn".into()),
            defs: None,
            refs: None,
            path: None,
            hist: None,
            file_type: None,
        };
        let suggestions = client.suggest(&req).await.unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].phrase, "func");
    }

    // ------------------------------------------------------------------
    // auth mode
    // ------------------------------------------------------------------

    #[test]
    fn auth_mode_bearer_clone() {
        let a = AuthMode::Bearer("tok".into());
        assert_eq!(a, AuthMode::Bearer("tok".into()));
    }

    #[test]
    fn auth_mode_basic_clone() {
        let a = AuthMode::Basic {
            username: "u".into(),
            password: "p".into(),
        };
        assert_ne!(a, AuthMode::None);
    }

    // ------------------------------------------------------------------
    // normalized_base_url
    // ------------------------------------------------------------------

    #[test]
    fn normalized_base_url_strips_trailing_slash() {
        let config = OpengrokClientConfig {
            base_url: "http://example.com/".into(),
            auth: AuthMode::None,
            timeout: Duration::from_secs(1),
            tls: TlsConfig::default(),
        };
        assert_eq!(config.normalized_base_url(), "http://example.com");
    }

    #[test]
    fn normalized_base_url_preserves_no_trailing_slash() {
        let config = OpengrokClientConfig {
            base_url: "http://example.com".into(),
            auth: AuthMode::None,
            timeout: Duration::from_secs(1),
            tls: TlsConfig::default(),
        };
        assert_eq!(config.normalized_base_url(), "http://example.com");
    }
}
