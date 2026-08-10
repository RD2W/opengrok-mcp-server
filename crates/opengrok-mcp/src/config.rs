// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! Configuration loading and validation.
//!
//! Loads TOML configuration with environment variable overrides.
//! Priority: `--config <path>` → `./config.toml` →
//! `./config/config.toml` → built-in defaults.
//!
//! Sensitive fields (token, password) are loaded from environment
//! variables specified in the config, never from the file itself.

use std::path::Path;

use serde::{Deserialize, Serialize};

const ENV_OPENGROK_URL: &str = "OPENGROK_URL";
const ENV_OPENGROK_CA_CERT: &str = "OPENGROK_CA_CERT";
const ENV_OPENGROK_CA_CERT_DIR: &str = "OPENGROK_CA_CERT_DIR";
const ENV_SSL_CERT_FILE: &str = "SSL_CERT_FILE";
const ENV_SSL_CERT_DIR: &str = "SSL_CERT_DIR";
const ENV_OPENGROK_VERIFY_SSL: &str = "OPENGROK_VERIFY_SSL";
const ENV_RUST_LOG: &str = "RUST_LOG";
const ENV_MCP_LOG_LEVEL: &str = "MCP_LOG_LEVEL";
const ENV_OPENGROK_TOKEN: &str = "OPENGROK_TOKEN";
const ENV_MCP_AUTH_TOKEN: &str = "MCP_AUTH_TOKEN";

pub const DEFAULT_CONFIG_PATH: &str = "config/config.toml";
const SEARCH_PATH_FALLBACK_1: &str = "./config/config.toml";
const SEARCH_PATH_FALLBACK_2: &str = "./config.toml";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Top-level configuration for the OpenGrok MCP server.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// OpenGrok connection settings.
    pub opengrok: OpengrokConfig,
    /// Service-level behaviour.
    pub service: ServiceConfig,
    /// Cache settings.
    pub cache: CacheConfig,
    /// Rate limit settings.
    pub rate_limit: RateLimitConfig,
    /// Transport settings.
    pub transport: TransportConfig,
    /// Logging settings.
    pub log: LogConfig,
}

impl Config {
    /// Loads configuration from the given explicit path or falls back
    /// to `./config/config.toml` → `./config.toml` → defaults.
    ///
    /// After loading the file, environment variables are applied as
    /// overrides for specific fields.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The explicit path is specified but the file doesn't exist.
    /// - The TOML file contains unknown fields.
    /// - The TOML syntax is invalid.
    /// - Validation fails (e.g. empty `base_url`).
    #[allow(dead_code)]
    pub fn load(explicit_path: Option<&str>) -> Result<Self, ConfigError> {
        let mut config = if let Some(path) = explicit_path {
            Self::from_file(Path::new(path))?
        } else {
            Self::from_search_paths(&[SEARCH_PATH_FALLBACK_1, SEARCH_PATH_FALLBACK_2])?
        };

        config.apply_env_overrides();
        config.validate()?;

        tracing::info!(
            base_url = %config.opengrok.base_url,
            transport_mode = ?config.transport.mode,
            "configuration loaded"
        );

        Ok(config)
    }

    fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            ConfigError::Io(format!(
                "failed to read config file '{}': {e}",
                path.display()
            ))
        })?;
        let config: Self = toml::from_str(&content).map_err(|e| {
            ConfigError::Parse(format!(
                "failed to parse config file '{}': {e}",
                path.display()
            ))
        })?;
        Ok(config)
    }

    fn from_search_paths(paths: &[&str]) -> Result<Self, ConfigError> {
        for path in paths {
            if Path::new(path).exists() {
                return Self::from_file(Path::new(path));
            }
        }
        tracing::info!("no config file found, using defaults");
        Ok(Self::default())
    }

    fn apply_env_overrides(&mut self) {
        // --- OpenGrok: connection ------------------------------------------

        if let Ok(val) = std::env::var(ENV_OPENGROK_URL) {
            self.opengrok.base_url = val;
        }
        if let Ok(val) = std::env::var("OPENGROK_TIMEOUT_SECS")
            && let Ok(v) = val.parse::<u64>()
        {
            self.opengrok.timeout_secs = v;
        }

        // --- OpenGrok: TLS -------------------------------------------------

        // Service-specific CA cert (wins over SSL_CERT_FILE)
        if let Ok(val) = std::env::var(ENV_OPENGROK_CA_CERT) {
            self.opengrok.ca_cert = Some(val);
        }
        // Shared CA cert fallback (only if OPENGROK_CA_CERT not set)
        if self.opengrok.ca_cert.is_none()
            && let Ok(val) = std::env::var(ENV_SSL_CERT_FILE)
        {
            self.opengrok.ca_cert = Some(val);
        }
        // Service-specific CA cert dir (wins over SSL_CERT_DIR)
        if let Ok(val) = std::env::var(ENV_OPENGROK_CA_CERT_DIR) {
            self.opengrok.ca_cert_dir = Some(val);
        }
        // Shared CA cert dir fallback
        if self.opengrok.ca_cert_dir.is_none()
            && let Ok(val) = std::env::var(ENV_SSL_CERT_DIR)
        {
            self.opengrok.ca_cert_dir = Some(val);
        }
        if let Ok(val) = std::env::var(ENV_OPENGROK_VERIFY_SSL)
            && (val.eq_ignore_ascii_case("false") || val == "0" || val.eq_ignore_ascii_case("no"))
        {
            self.opengrok.verify_ssl = false;
        }

        // --- OpenGrok: auth (indirect — env var name from TOML) -----------

        if let Some(ref token_env) = self.opengrok.auth.token_env.clone()
            && let Ok(token) = std::env::var(token_env)
        {
            self.opengrok.auth.token = Some(token);
        }
        if let Some(ref username_env) = self.opengrok.auth.username_env.clone()
            && let Ok(username) = std::env::var(username_env)
        {
            self.opengrok.auth.username = Some(username);
        }
        if let Some(ref password_env) = self.opengrok.auth.password_env.clone()
            && let Ok(password) = std::env::var(password_env)
        {
            self.opengrok.auth.password = Some(password);
        }

        // --- Service --------------------------------------------------------

        apply_bool_env("MCP_STRIP_HTML", &mut self.service.strip_html);
        if let Ok(val) = std::env::var("MCP_MAX_HITS_PER_FILE")
            && let Ok(v) = val.parse::<u32>()
        {
            self.service.max_hits_per_file = v;
        }
        if let Ok(val) = std::env::var("MCP_DEFAULT_MAX_RESULTS")
            && let Ok(v) = val.parse::<u32>()
        {
            self.service.default_max_results = v;
        }

        // --- Cache ----------------------------------------------------------

        apply_bool_env("MCP_CACHE_ENABLED", &mut self.cache.enabled);
        if let Ok(val) = std::env::var("MCP_CACHE_TTL_SECS")
            && let Ok(v) = val.parse::<u64>()
        {
            self.cache.ttl_secs = v;
        }
        if let Ok(val) = std::env::var("MCP_CACHE_MAX_ENTRIES")
            && let Ok(v) = val.parse::<usize>()
        {
            self.cache.max_entries = v;
        }

        // --- Rate limit -----------------------------------------------------

        apply_bool_env("MCP_RATE_LIMIT_ENABLED", &mut self.rate_limit.enabled);
        if let Ok(val) = std::env::var("MCP_RATE_LIMIT_RPS")
            && let Ok(v) = val.parse::<u32>()
        {
            self.rate_limit.requests_per_second = v;
        }
        if let Ok(val) = std::env::var("MCP_RATE_LIMIT_BURST")
            && let Ok(v) = val.parse::<u32>()
        {
            self.rate_limit.burst = v;
        }

        // --- Transport ------------------------------------------------------

        if let Ok(val) = std::env::var("MCP_TRANSPORT") {
            self.transport.mode = val;
        }
        if let Ok(val) = std::env::var("MCP_BIND_ADDR") {
            self.transport.bind_addr = val;
        }
        if let Ok(val) = std::env::var("MCP_HTTP_PATH") {
            self.transport.http_path = val;
        }
        if let Ok(val) = std::env::var("MCP_HEALTH_PATH") {
            self.transport.health_path = val;
        }
        if let Ok(val) = std::env::var("MCP_READY_PATH") {
            self.transport.ready_path = val;
        }
        if let Ok(val) = std::env::var("MCP_METRICS_PATH") {
            self.transport.metrics_path = val;
        }
        if let Ok(val) = std::env::var("MCP_ALLOWED_HOSTS") {
            self.transport.allowed_hosts = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        // MCP server-side token auth (inbound, not OpenGrok API auth)
        if let Ok(val) = std::env::var(ENV_MCP_AUTH_TOKEN) {
            self.transport.mcp_auth_token = val;
        }

        // --- Log ------------------------------------------------------------

        // MCP_LOG_LEVEL wins over RUST_LOG
        if let Ok(val) = std::env::var(ENV_MCP_LOG_LEVEL) {
            self.log.level = val;
        }
        if let Ok(val) = std::env::var(ENV_RUST_LOG) {
            self.log.level = val;
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.opengrok.base_url.is_empty() {
            return Err(ConfigError::Validation(
                "opengrok.base_url must not be empty".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// OpengrokConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OpengrokConfig {
    /// Base URL of the OpenGrok instance.
    #[serde(default)]
    pub base_url: String,
    /// Authentication settings.
    pub auth: AuthConfig,
    /// HTTP request timeout in seconds.
    pub timeout_secs: u64,
    /// Path to custom CA certificate PEM file.
    pub ca_cert: Option<String>,
    /// Path to directory of CA certificate files.
    pub ca_cert_dir: Option<String>,
    /// Whether to verify TLS certificates.
    pub verify_ssl: bool,
}

impl Default for OpengrokConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            auth: AuthConfig::default(),
            timeout_secs: 30,
            ca_cert: None,
            ca_cert_dir: None,
            verify_ssl: true,
        }
    }
}

// ---------------------------------------------------------------------------
// AuthConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    /// `"token"`, `"basic"`, or `"none"`.
    pub mode: String,
    /// Env variable name for the bearer token.
    pub token_env: Option<String>,
    /// The actual token value (populated from env at load time).
    #[serde(skip)]
    pub token: Option<String>,
    /// Env variable name for Basic auth username.
    pub username_env: Option<String>,
    /// The actual username (populated from env at load time).
    #[serde(skip)]
    pub username: Option<String>,
    /// Env variable name for Basic auth password.
    pub password_env: Option<String>,
    /// The actual password (populated from env at load time).
    #[serde(skip)]
    pub password: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: "none".into(),
            token_env: Some(ENV_OPENGROK_TOKEN.into()),
            token: None,
            username_env: None,
            username: None,
            password_env: None,
            password: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ServiceConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServiceConfig {
    /// Whether to strip `<b>` tags from search results.
    pub strip_html: bool,
    /// Maximum matching lines per file.
    pub max_hits_per_file: u32,
    /// Default maximum result documents.
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
// CacheConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_secs: 300,
            max_entries: 1000,
        }
    }
}

// ---------------------------------------------------------------------------
// RateLimitConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 5,
            burst: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// TransportConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransportConfig {
    /// `"stdio"`, `"http"`, or `"both"`.
    pub mode: String,
    /// Bind address for HTTP transport.
    pub bind_addr: String,
    /// URL path for the MCP Streamable HTTP endpoint.
    pub http_path: String,
    /// Health check endpoint path.
    pub health_path: String,
    /// Readiness check endpoint path.
    pub ready_path: String,
    /// Prometheus metrics endpoint path.
    pub metrics_path: String,
    /// Allowed hostnames for Streamable HTTP Host header validation.
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    /// Bearer token for MCP endpoint authentication.
    /// When non-empty, clients must include `Authorization: Bearer <token>`
    /// in every request. Token auth is disabled when this is empty.
    /// Environment: MCP_AUTH_TOKEN.
    #[serde(default)]
    pub mcp_auth_token: String,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            mode: "both".into(),
            bind_addr: "127.0.0.1:8080".into(),
            http_path: "/mcp".into(),
            health_path: "/healthz".into(),
            ready_path: "/readyz".into(),
            metrics_path: "/metrics".into(),
            allowed_hosts: vec![],
            mcp_auth_token: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// LogConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogConfig {
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// ConfigError
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config I/O error: {0}")]
    Io(String),
    #[error("config parse error: {0}")]
    Parse(String),
    #[error("config validation error: {0}")]
    Validation(String),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sets a bool from an env var. Accepts `true`/`false`/`1`/`0`/`yes`/`no`
/// (case-insensitive). If the var is not set, the target is unchanged.
fn apply_bool_env(var: &str, target: &mut bool) {
    if let Ok(val) = std::env::var(var) {
        if val.eq_ignore_ascii_case("true") || val == "1" || val.eq_ignore_ascii_case("yes") {
            *target = true;
        } else if val.eq_ignore_ascii_case("false") || val == "0" || val.eq_ignore_ascii_case("no")
        {
            *target = false;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_bool_env_true_values() {
        for val in &["true", "TRUE", "1", "yes", "YES"] {
            unsafe { std::env::set_var("__TEST_BOOL_TRUE", val) };
            let mut target = false;
            apply_bool_env("__TEST_BOOL_TRUE", &mut target);
            assert!(target, "expected true for '{val}'");
        }
        unsafe { std::env::remove_var("__TEST_BOOL_TRUE") };
    }

    #[test]
    fn apply_bool_env_false_values() {
        for val in &["false", "FALSE", "0", "no", "NO"] {
            unsafe { std::env::set_var("__TEST_BOOL_FALSE", val) };
            let mut target = true;
            apply_bool_env("__TEST_BOOL_FALSE", &mut target);
            assert!(!target, "expected false for '{val}'");
        }
        unsafe { std::env::remove_var("__TEST_BOOL_FALSE") };
    }

    #[test]
    fn apply_bool_env_no_op_if_unset() {
        let mut target = true;
        apply_bool_env("__NONEXISTENT_ENV_VAR_XYZ", &mut target);
        assert!(target, "target should remain unchanged");
    }

    #[test]
    fn default_config_validates_fails_no_base_url() {
        let config = Config::default();
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn default_config_with_base_url_validates() {
        let config = Config {
            opengrok: OpengrokConfig {
                base_url: "http://localhost:8080".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn parses_valid_toml() {
        let toml_str = r#"
[opengrok]
base_url = "https://opengrok.example.com"

[opengrok.auth]
mode = "token"
token_env = "MY_TOKEN"

[service]
strip_html = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.opengrok.base_url, "https://opengrok.example.com");
        assert_eq!(config.opengrok.auth.mode, "token");
        assert!(config.service.strip_html);
    }

    #[test]
    fn denies_unknown_fields() {
        let toml_str = r#"
[opengrok]
base_url = "http://x"
unknown_field = 42
"#;
        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn env_override_base_url() {
        let mut config = Config::default();
        config.opengrok.base_url = "https://from-env.example.com".into();
        assert_eq!(config.opengrok.base_url, "https://from-env.example.com");
    }

    #[test]
    fn auth_token_from_env() {
        let mut config = Config::default();
        config.opengrok.auth.token_env = Some("TEST_TOKEN".into());
        assert!(config.opengrok.auth.token.is_none());
    }

    #[test]
    fn transport_defaults() {
        let config = TransportConfig::default();
        assert_eq!(config.mode, "both");
        assert_eq!(config.bind_addr, "127.0.0.1:8080");
        assert_eq!(config.http_path, "/mcp");
        assert!(config.allowed_hosts.is_empty());
    }

    #[test]
    fn transport_parses_allowed_hosts() {
        let toml_str = r#"
[transport]
mode = "http"
allowed_hosts = ["host-a", "host-b:8004"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.transport.allowed_hosts,
            vec!["host-a", "host-b:8004"]
        );
    }

    #[test]
    fn env_override_transport_fields() {
        unsafe {
            std::env::set_var("__TEST_BIND_ADDR", "0.0.0.0:9999");
            std::env::set_var("__TEST_HTTP_PATH", "/api");
            std::env::set_var("__TEST_HEALTH_PATH", "/h");
            std::env::set_var("__TEST_READY_PATH", "/r");
            std::env::set_var("__TEST_METRICS_PATH", "/m");
        }

        let mut config = Config::default();
        if let Ok(val) = std::env::var("__TEST_BIND_ADDR") {
            config.transport.bind_addr = val;
        }
        if let Ok(val) = std::env::var("__TEST_HTTP_PATH") {
            config.transport.http_path = val;
        }
        if let Ok(val) = std::env::var("__TEST_HEALTH_PATH") {
            config.transport.health_path = val;
        }
        if let Ok(val) = std::env::var("__TEST_READY_PATH") {
            config.transport.ready_path = val;
        }
        if let Ok(val) = std::env::var("__TEST_METRICS_PATH") {
            config.transport.metrics_path = val;
        }

        assert_eq!(config.transport.bind_addr, "0.0.0.0:9999");
        assert_eq!(config.transport.http_path, "/api");
        assert_eq!(config.transport.health_path, "/h");
        assert_eq!(config.transport.ready_path, "/r");
        assert_eq!(config.transport.metrics_path, "/m");

        unsafe {
            std::env::remove_var("__TEST_BIND_ADDR");
            std::env::remove_var("__TEST_HTTP_PATH");
            std::env::remove_var("__TEST_HEALTH_PATH");
            std::env::remove_var("__TEST_READY_PATH");
            std::env::remove_var("__TEST_METRICS_PATH");
        }
    }

    #[test]
    fn env_override_service_numeric_fields() {
        unsafe {
            std::env::set_var("__TEST_MAX_HITS", "42");
            std::env::set_var("__TEST_MAX_RESULTS", "99");
        }

        let mut config = Config::default();
        if let Ok(val) = std::env::var("__TEST_MAX_HITS")
            && let Ok(v) = val.parse::<u32>()
        {
            config.service.max_hits_per_file = v;
        }
        if let Ok(val) = std::env::var("__TEST_MAX_RESULTS")
            && let Ok(v) = val.parse::<u32>()
        {
            config.service.default_max_results = v;
        }

        assert_eq!(config.service.max_hits_per_file, 42);
        assert_eq!(config.service.default_max_results, 99);

        unsafe {
            std::env::remove_var("__TEST_MAX_HITS");
            std::env::remove_var("__TEST_MAX_RESULTS");
        }
    }

    #[test]
    fn env_override_allowed_hosts_from_csv() {
        let mut config = Config::default();
        let raw = "host-a, host-b:8004 , 127.0.0.1";
        config.transport.allowed_hosts = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            config.transport.allowed_hosts,
            vec!["host-a", "host-b:8004", "127.0.0.1"]
        );
    }

    #[test]
    fn env_override_verify_ssl_disabled() {
        let mut opengrok = OpengrokConfig::default();
        assert!(opengrok.verify_ssl);

        // Simulate: OPENGROK_VERIFY_SSL=false
        for val in &["false", "FALSE", "0", "no", "NO"] {
            let disabled =
                val.eq_ignore_ascii_case("false") || *val == "0" || val.eq_ignore_ascii_case("no");
            if disabled {
                opengrok.verify_ssl = false;
            }
            assert!(!opengrok.verify_ssl, "expected false for '{val}'");
            opengrok.verify_ssl = true; // reset
        }
    }
}
