// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! OpenGrok MCP server — entry point.
//!
//! Loads configuration, initializes tracing, builds the application
//! service, and starts the selected transport (stdio / HTTP / both).

mod config;
mod health;
mod mcp;
mod transport;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use opengrok_core::application::{OpengrokService, ServiceConfig};
use opengrok_core::infrastructure::client::{AuthMode, OpengrokClient, OpengrokClientConfig};
use opengrok_core::infrastructure::tls::TlsConfig;
use tracing_subscriber::EnvFilter;

use crate::config::{Config, DEFAULT_CONFIG_PATH};
use crate::transport::run_transport;

/// Custom version string with full build metadata.
const VERSION_TEXT: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\n",
    "author:  ",
    env!("CARGO_PKG_AUTHORS"),
    "\n",
    "commit:  ",
    env!("GIT_HASH"),
    "\n",
    "built:   ",
    env!("BUILD_DATE"),
    "\n",
    "target:  ",
    env!("BUILD_TARGET"),
);

#[derive(Parser)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(version = VERSION_TEXT)]
#[command(about = env!("CARGO_PKG_DESCRIPTION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Args {
    /// Path to configuration file
    #[arg(short = 'c', long = "config", default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load config
    let config = Config::load(args.config.to_str())?;

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log.level)),
        )
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "opengrok-mcp starting");

    // Build auth mode
    let auth = match config.opengrok.auth.mode.as_str() {
        "token" => {
            let token = config.opengrok.auth.token.clone().ok_or_else(|| {
                anyhow::anyhow!(
                    "token auth mode requires {} to be set",
                    config
                        .opengrok
                        .auth
                        .token_env
                        .as_deref()
                        .unwrap_or("OPENGROK_TOKEN")
                )
            })?;
            AuthMode::Bearer(token)
        }
        "basic" => {
            let username = config.opengrok.auth.username.clone().ok_or_else(|| {
                anyhow::anyhow!("basic auth mode requires username env to be set")
            })?;
            let password = config.opengrok.auth.password.clone().unwrap_or_default();
            AuthMode::Basic { username, password }
        }
        "none" => AuthMode::None,
        other => anyhow::bail!("unknown auth mode: '{other}' (expected token, basic, or none)"),
    };

    // Build HTTP client
    let client_config = OpengrokClientConfig {
        base_url: config.opengrok.base_url.clone(),
        auth,
        timeout: Duration::from_secs(config.opengrok.timeout_secs),
        tls: TlsConfig {
            verify_ssl: config.opengrok.verify_ssl,
            ca_cert: config.opengrok.ca_cert.clone(),
            ca_cert_dir: config.opengrok.ca_cert_dir.clone(),
        },
    };

    let client = OpengrokClient::new(client_config)?;

    // Build service
    let mut service = OpengrokService::new(
        client,
        ServiceConfig {
            strip_html: config.service.strip_html,
            max_hits_per_file: config.service.max_hits_per_file,
            default_max_results: config.service.default_max_results,
        },
    );

    if config.cache.enabled {
        service = service.with_cache(
            Duration::from_secs(config.cache.ttl_secs),
            config.cache.max_entries,
        );
    }

    if config.rate_limit.enabled {
        service = service.with_rate_limit(
            config.rate_limit.requests_per_second,
            config.rate_limit.burst,
        );
    }

    // Run transport
    run_transport(&config, service).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_default_config() {
        let args = Args::parse_from(["opengrok-mcp"]);
        assert_eq!(args.config, PathBuf::from("config/config.toml"));
    }

    #[test]
    fn parse_custom_config_long() {
        let args = Args::parse_from(["opengrok-mcp", "--config", "/etc/opengrok.toml"]);
        assert_eq!(args.config, PathBuf::from("/etc/opengrok.toml"));
    }

    #[test]
    fn parse_custom_config_short() {
        let args = Args::parse_from(["opengrok-mcp", "-c", "/etc/opengrok.toml"]);
        assert_eq!(args.config, PathBuf::from("/etc/opengrok.toml"));
    }

    #[test]
    fn parse_custom_config_equals() {
        let args = Args::parse_from(["opengrok-mcp", "--config=/etc/opengrok.toml"]);
        assert_eq!(args.config, PathBuf::from("/etc/opengrok.toml"));
    }

    #[test]
    fn version_text_contains_expected_fields() {
        assert!(VERSION_TEXT.contains("author:"), "missing author field");
        assert!(VERSION_TEXT.contains("commit:"), "missing commit field");
        assert!(VERSION_TEXT.contains("built:"), "missing built field");
        assert!(VERSION_TEXT.contains("target:"), "missing target field");
    }

    #[test]
    fn version_text_has_non_empty_hash() {
        let commit_line = VERSION_TEXT
            .lines()
            .find(|l| l.starts_with("commit:"))
            .expect("commit line not found");
        let hash = commit_line.trim_start_matches("commit:").trim();
        assert!(!hash.is_empty(), "commit hash should not be empty");
    }

    #[test]
    fn version_text_build_date_is_iso8601() {
        let date_line = VERSION_TEXT
            .lines()
            .find(|l| l.starts_with("built:"))
            .expect("built line not found");
        let date = date_line.trim_start_matches("built:").trim();
        // ISO 8601: 2026-07-23T14:30:00Z
        assert!(date.contains('T'), "missing T separator in ISO 8601 date");
        assert!(date.ends_with('Z'), "missing Z suffix in ISO 8601 date");
        assert_eq!(
            date.len(),
            20,
            "expected ISO 8601 length (YYYY-MM-DDTHH:MM:SSZ)"
        );
    }

    #[test]
    fn version_text_target_is_not_empty() {
        let target_line = VERSION_TEXT
            .lines()
            .find(|l| l.starts_with("target:"))
            .expect("target line not found");
        let target = target_line.trim_start_matches("target:").trim();
        assert!(!target.is_empty(), "target should not be empty");
        assert!(target.contains('-'), "target triple should contain hyphens");
    }
}
