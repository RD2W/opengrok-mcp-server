# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to
follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.2] — 2026-09-30

### Changed

- **MSRV raised to Rust 1.98** (`rust-version = "1.98"`); the Docker builder image
  moves to `rust:1.98.1-alpine3.24` to match the toolchain used locally and in CI.
- **`rmcp` 3.1.3 → 3.5.0** — the jump through 3.2.0/3.4.0 required adapting to the
  `ServerInfo` → `ServerConfig` rename; 3.4.1 → 3.5.0 needed no source changes.
- **Transitive dependency refresh** — `hyper-util` 0.1.21, `tokio-rustls` 0.26.6,
  `rustls-platform-verifier` 0.7.1, `quinn-proto`/`quinn-udp`, `cc` 1.5.1,
  `smallvec` 1.16.2, `zerocopy` 0.8.59 and the wasm-only
  `js-sys`/`wasm-bindgen*`/`web-sys` set.
- **Other dependency bumps** — `rustls` 0.23.43 → 0.23.45, `reqwest` 0.13.5,
  `thiserror` 2.0.21, `clap` 4.6.7, `toml` 1.1.6, `lru` 0.18.5.
- `matchit` stays at 0.8.4 (pinned by `axum 0.8.x`); the constraint is now documented
  in the workspace manifest.

### Removed

- **`rustls-pemfile`** — unmaintained (RUSTSEC-2025-0134, repository archived in
  August 2025). PEM parsing now uses `CertificateDer::pem_slice_iter()` from
  `rustls-pki-types`, which `rustls 0.23` already re-exports, so no new dependency was
  added; the orphaned `base64 0.22.1` went with it. A scan of the direct dependencies
  against OSV reports **no advisories**.

### Tests

- **PEM parsing parity test** — a bundle containing a private key plus a certificate
  must still yield exactly one certificate, locking the behaviour across the
  `rustls-pemfile` → `rustls-pki-types` migration. Total: **185 tests** (was 184).

### Documentation

- Status/version references updated (v1.2.1 → v1.2.2, test count 184 → 185) in the
  EN/RU overview, installation pages, and README; the Rust badge now reads 1.98+.

## [1.2.1] — 2026-08-19

### Fixed

- **Container health restored** — the Dockerfile now exposes `HEALTHCHECK_PORT`
  (default `8080`) and `HEALTHCHECK_PATH` (default `/healthz`) as build args / env
  vars. The image healthcheck probes `127.0.0.1:${HEALTHCHECK_PORT}${HEALTHCHECK_PATH}`
  inside the container, so overriding `bind_addr` no longer leaves the container
  reporting `unhealthy`.

### Added

- **Makefile** — `make dev`, `make docker-build`, `make docker-push`, `make test`
  and related targets for local development and Docker workflows.

### Tests

- **Health endpoint contract tests** — integration coverage for `/healthz`,
  `/readyz`, and `/metrics` on the Streamable HTTP transport.

### Changed

- **34 dependency updates** to latest semver-compatible versions (rmcp 3.1.3,
  futures 0.3.34, quinn-proto, rustls-webpki, icu 2.3, uuid, etc.).

### Documentation

- Fixed the `MCP_AUTH_TOKEN` env var name in docs (was `MCP_TOKEN`).
- Status/version references updated (v1.1.1 → v1.2.1, test count 170 → 184) in
  EN/RU overview, installation, and README.

## [1.2.0] — 2026-08-10

### Added

- **Environment variable overrides for all config fields**: every `[opengrok]`,
  `[server]`, `[cache]`, and `[rate_limit]` setting can now be overridden via
  `OPENGROK_*` / `MCP_SERVER_*` / `CACHE_*` / `RATE_LIMIT_*` env vars.
- **Tool call metrics and LRU cache**: tool call counter, error counter, and
  per-tool duration tracking; in-memory TTL cache replaced with LRU eviction
  for bounded memory usage.

### Fixed

- **Bind to loopback by default**: server now listens on `127.0.0.1` instead of
  `0.0.0.0`, preventing accidental external exposure.
- **MCP token auth middleware**: token extraction and validation aligned with
  gerrit-mcp-server pattern — simpler, more robust.
- **Secrets redacted in `AuthMode::Debug`**: Bearer tokens no longer leak into
  debug output.
- **`list_project_files` fixed**: replaced recursive file tree API with
  directory listing endpoint, avoiding explosions on large repositories.
- **Response size guard**: responses exceeding 50 MiB are rejected with a
  descriptive error instead of consuming unbounded memory.
- **Increased timeout for large OpenGrok responses**: prevents premature
  disconnection on slow or heavily-loaded backends.
- **OpenGrok API wrapper objects**: `suggest` and `list_project_files` now
  correctly parse JSON responses wrapped in `{results, ...}` objects.

### Changed

- **Token auth refactored**: shared MCP authentication service extracted and
  simplified to match the established gerrit-mcp-server pattern.
- **Module structure**: `mcp` and `domain` modules split into focused
  sub-modules for better maintainability.
- **Dependencies**: reqwest unpinned (0.13.4 → 0.13), lru upgraded (0.15 → 0.18).

### Development

- **CI**: Dependabot target-branch set to `dev`.
- **Removed unused dev-dependencies**: `wiremock`, `tempfile` from crates where
  they were not imported.
- **Documentation**: config examples and `.env.example` rewritten in English;
  README and docs synced with current defaults.

## [1.1.1] — 2026-08-09

### Changed

- **rmcp upgraded to 3.1.2** (was 3.0.1) — MCP protocol conformance fixes:
  stateless request routing for 2026-07-28, MRTR round-trip support,
  per-request protocol metadata validation, cancel-safe receive.

### Fixed

- **HTTP transport: disabled legacy session mode** via
  `StreamableHttpServerConfig::with_legacy_session_mode(false)`.
  rmcp 3.1.x requires this when using `NeverSessionManager` —
  otherwise all requests are routed through the legacy session
  creation path, which `NeverSessionManager` rejects.

### Added

- **Docker: UPX binary compression** for smaller image size (Dockerfile + CI release workflow).
- **CI: install-upx composite action** and UPX step in binary release builds.

### Changed

- **Updated 28 dependencies** to latest semver-compatible versions
  (base64, clap, thiserror, zerocopy, async-trait, aws-lc-rs, etc.).

## [1.1.0] — 2026-07-31

### Changed

- **rmcp upgraded to 3.0** (was 2.2) — MCP 2026-07-28 protocol support:
  stateless discovery, protocol negotiation, multi round-trip requests (MRTR).
  Tool return types handled automatically by `#[tool_router]` macro.
- **HTTP transport: swapped `LocalSessionManager` → `NeverSessionManager`**
  for fully stateless Streamable HTTP (no session tracking, no GET/DELETE).
- **Server announces both protocol versions**: 2026-07-28 (preferred) and
  2025-11-25 (legacy fallback) via `supported_protocol_versions()`.

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

[1.2.2]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.2.2
[1.2.1]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.2.1
[1.2.0]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.2.0
[1.1.1]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.1.1
[1.1.0]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.1.0
[1.0.0]: https://github.com/RD2W/opengrok-mcp-server/releases/tag/v1.0.0
