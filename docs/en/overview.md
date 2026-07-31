# Overview

`opengrok-mcp` is an MCP (Model Context Protocol) server that bridges LLM clients to
[OpenGrok](https://oracle.github.io/opengrok/) code search. It was designed for
**AOSP 15-scale** codebases — tens of millions of lines of code across hundreds of
projects — and handles the latency, result size, and authentication challenges that
come with that scale.

Next: [Installation →](./installation.md)

---

## What problem does it solve?

LLM agents (Claude Desktop, Codex, etc.) need to search and read source code in
large monorepos. OpenGrok provides a REST API for full-text search, file browsing,
history, and annotation, but its raw API is not LLM-friendly:

- HTML tags in results (`<b>match</b>`) need stripping
- Pagination needs to be handled and communicated to the LLM via `has_more` hints
- Large result sets need capping and caching for latency
- The API has quirks (`null` tags, empty `lineNumbers`) that need normalisation
- Authentication and TLS with corporate CAs must be configured

`opengrok-mcp` wraps all of this into 25 clean MCP tools with proper error
handling, rate limiting, and result formatting.

---

## Features

### 25 MCP tools — full OpenGrok REST API coverage

| Category | Tools |
|---|---|
| Search (7) | `search_code`, `search_definition`, `search_references`, `search_file_path`, `search_history`, `advanced_search`, `suggest` |
| Files (5) | `get_file_content`, `get_file_definitions`, `get_file_genre`, `get_history`, `get_annotation` |
| Navigation (8) | `list_directory`, `list_indexed_projects`, `list_all_projects`, `list_groups`, `get_group_projects`, `list_project_files`, `list_project_repos`, `get_project_property` |
| System (5) | `get_repo_property`, `get_suggest_config`, `get_index_time`, `get_opengrok_version`, `health_check` |

Each tool declares its parameters via JSON Schema (schemars), so LLM clients
automatically know the expected inputs and outputs — no manual prompt engineering
needed.

### Dual transport

| Mode | Use case |
|---|---|
| **stdio** | Direct process launch: `docker exec`, Claude Desktop local subprocess, debugging |
| **Streamable HTTP** | Network deployment: remote server, multiple clients, health checks, metrics |

The `both` mode runs stdio and HTTP simultaneously.

### Flexible authentication

| Mode | Description |
|---|---|
| `token` | Bearer token from an environment variable |
| `basic` | HTTP Basic Auth (username/password from env vars) |
| `none` | No authentication header — for open instances |

Credentials are **never** stored in the config file — only environment variable names.

### TLS with custom CAs

Corporate or self-signed CA certificates are supported through:

- `OPENGROK_CA_CERT` / `SSL_CERT_FILE` — a single PEM file
- `SSL_CERT_DIR` — a directory of certificate files
- `config/certs/` directory (mounted read-only in Docker)

### DNS rebinding protection

When running in HTTP mode, rmcp validates the `Host` header against a configurable
`allowed_hosts` list. Requests with non-matching hosts receive **403 Forbidden**.
This prevents DNS rebinding attacks when the server is exposed on a network.

### Optimised for AOSP-scale codebases

| Feature | Purpose |
|---|---|
| **HTML tag stripping** | Removes `<b>` tags from search results — cleaner output for LLMs |
| **Result capping** | `max_hits_per_file` limits matching lines per file |
| **In-memory cache** | TTL-based cache with configurable size, avoids repeated API calls |
| **Rate limiting** | Token-bucket limiter protects the OpenGrok backend from overload |
| **Pagination hints** | `has_more` field in responses tells the LLM when more results are available |

### Health & metrics

| Endpoint | Purpose |
|---|---|
| `/healthz` | Liveness — always returns 200 if the server is running |
| `/readyz` | Readiness — 200 when config is loaded and OpenGrok is reachable |
| `/metrics` | Prometheus-format metrics (request counts, latencies, cache stats) |

### Docker

Multi-stage build producing a **~35 MB** Alpine-based image. Docker Compose config
for local development and remote deployment.

---

## Current status

**v1.0.0.** The core HTTP client, all 25 MCP tools, dual transport, caching, rate
limiting, TLS, health endpoints, and Docker packaging are implemented and covered
by **158 tests**. Supports MCP 2026-07-28 protocol (stateless Streamable HTTP,
protocol negotiation) with legacy 2025-11-25 fallback.
