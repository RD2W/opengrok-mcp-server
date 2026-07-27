# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-07-27

First stable release. 🚀

### MCP Tools (25 total)

- **Search (7):** `search_code`, `search_definition`, `search_references`,
  `search_file_path`, `search_history`, `advanced_search`, `suggest`
- **Files (5):** `get_file_content`, `get_file_definitions`, `get_file_genre`,
  `get_history`, `get_annotation`
- **Navigation (8):** `list_directory`, `list_indexed_projects`,
  `list_all_projects`, `list_groups`, `get_group_projects`, `list_project_files`,
  `list_project_repos`, `get_project_property`
- **Other (5):** `get_repo_property`, `get_suggest_config`, `get_index_time`,
  `get_opengrok_version`, `health_check`

All tools expose JSON Schema via `schemars` for MCP client consumption.

### Authentication

- Bearer token authentication
- HTTP Basic authentication (username + password)
- Custom CA certificates via `OPENGROK_CA_CERT` / `SSL_CERT_FILE`
- Optional TLS verification disable for trusted internal networks
- All credentials sourced from environment variables — never stored in config

### Transport

- **stdio** — local subprocess / Claude Desktop / `docker exec`
- **Streamable HTTP** — axum + rmcp for multi-client remote deployments
- **Both** — simultaneous stdio + HTTP for debugging
- DNS rebinding protection via `allowed_hosts`

### TLS

- rustls-based TLS (pure Rust, no OpenSSL dependency)
- System trust store integration via `rustls-native-certs`
- Custom CA certificate file or directory support
- Optional verification disable

### Caching & Rate Limiting

- In-memory TTL cache (`DashMap`-based) with configurable TTL and max entries
- Token-bucket rate limiting via `governor` (GCRA algorithm)
- Both applied transparently via `OpengrokService` decorator pattern

### Health & Metrics

- `GET /healthz` — liveness probe
- `GET /readyz` — readiness probe (with OpenGrok connectivity check)
- `GET /metrics` — Prometheus-formatted metrics (`tool_calls_total`,
  `tool_errors_total`, `uptime_seconds`)

### Deployment

- Multi-stage Docker image (~35 MB, Alpine 3.24)
- Multi-arch support: `linux/amd64`, `linux/arm64`
- Docker Compose files for local build and Docker Hub images
- OCI labels, non-root user, healthcheck in image

### Documentation

- Bilingual documentation (EN + RU): overview, installation, usage (config reference
  + all 25 MCP tools with parameter schemas), architecture, development guide
- Bilingual README.md
- Annotated configuration example (`config/config.example.toml`)
- API coverage matrix mapping all OpenGrok endpoints to MCP tools

### CI/CD

- GitHub Actions: format (`cargo fmt`), lint (`clippy -D warnings`), test, release build
- Multi-arch Docker release on tag push
- Pre-built images on Docker Hub (`rd2w/opengrok-mcp`)

### Testing

- 158 tests (unit, integration, MCP tool pipeline)
- Loopback HTTP client integration tests
- `MockOpengrokRepository` for full pipeline testing

## [0.1.0] — 2026-07-26

Initial pre-release.

[1.0.0]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.0.0
