# Usage

Previous: [← Installation](./installation.md)
Next: [Architecture →](./architecture.md)

---

## Version

```bash
opengrok-mcp --version
```

Output includes version, author, commit hash, build date, and target platform.

> **Docker note:** When building the Docker image manually without `--build-arg GIT_HASH`,
> the commit hash will show as `unknown`. For correct metadata, pass the hash:
>
> ```bash
> docker build --build-arg GIT_HASH=$(git rev-parse HEAD) .
> ```

## Configuration reference

The configuration file is `config/config.toml`. See `config/config.example.toml` for
the annotated template. Environment variables override specific fields (listed below).

### `[opengrok]` — connection

| Field | Env var | Default | Description |
|---|---|---|---|
| `base_url` | `OPENGROK_URL` | `""` | **Required.** OpenGrok base URL (e.g. `https://opengrok.example.com`) |
| `timeout_secs` | — | `60` | HTTP request timeout — AOSP searches can be slow |
| `ca_cert` | `OPENGROK_CA_CERT` / `SSL_CERT_FILE` | `"./config/certs/russian_trusted_root_ca_pem.crt"` | Custom CA PEM path |
| `ca_cert_dir` | `SSL_CERT_DIR` | — | Directory of CA certs |
| `verify_ssl` | `OPENGROK_VERIFY_SSL=false` | `true` | Enable/disable TLS verification |

### `[opengrok.auth]` — authentication

| Field | Description |
|---|---|
| `mode` | `"token"`, `"basic"`, or `"none"` |
| `token_env` | Env var name for the Bearer token (default: `OPENGROK_TOKEN`) |
| `username_env` | Env var name for Basic auth username (default: `OPENGROK_USERNAME`) |
| `password_env` | Env var name for Basic auth password (default: `OPENGROK_PASSWORD`) |

Credentials are never stored in the config file — only the env var names.

### `[service]` — behaviour

| Field | Default | Description |
|---|---|---|
| `strip_html` | `true` | Strip `<b>` HTML tags from search lines |
| `max_hits_per_file` | `10` | Max matching lines per file in results |
| `default_max_results` | `25` | Default result limit when client doesn't specify |

### `[cache]` — in-memory cache

| Field | Default | Description |
|---|---|---|
| `enabled` | `false` | Enable/disable cache |
| `ttl_secs` | `300` | Entry lifetime |
| `max_entries` | `1000` | Max cached responses |

### `[rate_limit]` — token bucket

| Field | Default | Description |
|---|---|---|
| `enabled` | `false` | Enable/disable rate limiting |
| `requests_per_second` | `5` | Sustained request rate |
| `burst` | `10` | Burst capacity |

### `[transport]` — server mode

| Field | Default | Description |
|---|---|---|
| `mode` | `"stdio"` | `"stdio"`, `"http"`, or `"both"` |
| `bind_addr` | `"0.0.0.0:8080"` | HTTP bind address |
| `http_path` | `"/mcp"` | MCP endpoint path |
| `health_path` | `"/healthz"` | Liveness endpoint |
| `ready_path` | `"/readyz"` | Readiness endpoint |
| `metrics_path` | `"/metrics"` | Prometheus metrics endpoint |
| `allowed_hosts` | `[]` | Allowed Host header values (DNS rebinding protection) |

### `[log]`

| Field | Default | Description |
|---|---|---|
| `level` | `"info"` | `trace`, `debug`, `info`, `warn`, `error` — overridden by `RUST_LOG` |

---

## Transport modes

### stdio mode

```toml
[transport]
mode = "stdio"
```

The server reads MCP messages from stdin and writes to stdout. Use this:

- With `docker exec` for containerised OpenGrok sidecars
- With Claude Desktop or other local MCP clients
- For debugging — easy to pipe test JSON messages

### HTTP mode (Streamable HTTP)

```toml
[transport]
mode = "http"
bind_addr = "0.0.0.0:8080"
```

The server starts an HTTP server with:

- MCP endpoint at the configured `http_path` (`/mcp`)
- Health check at `/healthz`
- Readiness check at `/readyz`
- Prometheus metrics at `/metrics`

Use this for multi-client deployments, remote access, or when the MCP client
doesn't support process spawning.

### both mode

```toml
[transport]
mode = "both"
```

Runs stdio and HTTP simultaneously. Useful for debugging HTTP deployments:
the stdio channel lets you inspect traffic while the HTTP server handles
production load.

---

## DNS rebinding protection

In HTTP mode, rmcp validates the `Host` header against `allowed_hosts`. Configure
it for your deployment:

```toml
# Docker — clients connect via container name
allowed_hosts = ["localhost", "127.0.0.1", "opengrok-mcp", "opengrok-mcp:8004"]

# Public deployment behind a reverse proxy
allowed_hosts = ["localhost", "mcp.example.com"]
```

Empty `allowed_hosts` uses rmcp defaults: `localhost`, `127.0.0.1`, `::1` only.

---

## Health endpoints

| Endpoint | Behaviour |
|---|---|
| `GET /healthz` | Always `200 OK` if the process is alive |
| `GET /readyz` | `200` when config is loaded and OpenGrok responds to a lightweight probe; `503` otherwise |
| `GET /metrics` | Prometheus text format — request counters, latencies, cache hits/misses |

### Docker health check

```yaml
healthcheck:
  test: ["CMD", "wget", "-qO-", "http://localhost:8080/healthz"]
  interval: 30s
  retries: 3
```

---

## MCP tools reference

The server exposes **25 tools** covering the full OpenGrok REST API.

### Search tools

| Tool | Description | Key parameters |
|---|---|---|
| `search_code` | Full-text search (Lucene syntax) | `query`, `project?`, `max_results` |
| `search_definition` | Find symbol definitions | `symbol`, `project?` |
| `search_references` | Find all references to a symbol | `symbol`, `project?` |
| `search_file_path` | Search for files by path glob | `path`, `project?` |
| `search_history` | Search file history/changelog | `hist`, `project?` |
| `advanced_search` | Advanced search (all fields, pagination, sorting) | `full?`, `def?`, `symbol?`, `path?`, `hist?`, `file_type?`, `project?`, `max_results?`, `start?`, `max_hits_per_file?`, `sort?` |
| `suggest` | Query autocomplete suggestions | `project`, `field`, `caret`, `full?`, `defs?`, `refs?`, `path?`, `file_type?` |

### File tools

| Tool | Description | Key parameters |
|---|---|---|
| `get_file_content` | Retrieve raw file content | `project`, `path` |
| `get_file_definitions` | List definitions (functions, classes) in a file | `path` |
| `get_file_genre` | File type (PLAIN, XREFABLE, IMAGE) | `path` |
| `get_history` | File revision history | `path`, `start?`, `max?` |
| `get_annotation` | Annotation (blame) for a file | `path` |

### Directory & project tools

| Tool | Description | Key parameters |
|---|---|---|
| `list_directory` | List directory contents | `path` |
| `list_indexed_projects` | List indexed projects | — |
| `list_all_projects` | List all projects (including non-indexed) | — |
| `list_groups` | List project groups | — |
| `get_group_projects` | Projects within a group (including subgroups) | `group` |
| `list_project_files` | List files in a project from index | `project` |
| `list_project_repos` | Repository paths for a project | `project` |
| `get_project_property` | Per-project property value | `project`, `name` |
| `get_repo_property` | Repository property (type, branch, remote) | `field`, `repository` |

### System tools

| Tool | Description | Key parameters |
|---|---|---|
| `get_suggest_config` | Suggester configuration | — |
| `get_index_time` | Last index time (ISO 8601) | — |
| `get_opengrok_version` | OpenGrok version string | — |
| `health_check` | Check OpenGrok liveness | — |

### Result format

All search results include:

```json
{
  "results": [...],
  "total_hits": 1423,
  "page": 1,
  "has_more": true,
  "duration_ms": 230
}
```

- `has_more: true` tells the LLM that more results are available — it can request the next page
- `duration_ms` is the server-side processing time, useful for latency debugging
- HTML tags are stripped from result lines when `strip_html = true`

### Pagination

When `has_more` is `true`, the client can request additional pages by setting
the `page` parameter:

```
Tool: search
Query: "init_boot_images"
Project: "aosp"
Page: 2
```

The server transparently handles OpenGrok's pagination mechanics and exposes
a simple page-based interface.
