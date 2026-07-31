# Architecture

Previous: [← Usage](./usage.md)
Next: [Development →](./development.md)

---

## Workspace structure

```
opengrok-mcp-server/
├── crates/
│   ├── opengrok-core/         # Library — no MCP dependencies
│   │   └── src/
│   │       ├── domain.rs      # Data models: SearchResult, FileContent, Project, …
│   │       ├── application.rs # Service layer: search, file ops, pagination logic
│   │       └── infrastructure/
│   │           ├── client.rs  # HTTP client for OpenGrok REST API
│   │           ├── tls.rs     # TLS configuration builder (rustls + native certs)
│   │           ├── cache.rs   # In-memory TTL cache (DashMap)
│   │           ├── rate_limit.rs # Token-bucket rate limiter (governor)
│   │           └── format.rs  # HTML tag stripping, result normalisation
│   └── opengrok-mcp/          # Binary — MCP server layer
│       └── src/
│           ├── main.rs        # Entry point, CLI args, logging init
│           ├── config.rs      # TOML config loading + env var overrides
│           ├── mcp/
│           │   ├── mod.rs     # MCP server setup, tool dispatch
│           │   └── tools.rs   # Tool definitions (JSON Schema via schemars)
│           ├── transport/
│           │   ├── mod.rs     # Transport abstraction
│           │   ├── stdio.rs   # stdin/stdout transport
│           │   └── http.rs    # Axum + rmcp Streamable HTTP transport
│           └── health.rs      # /healthz, /readyz, /metrics endpoints
├── config/
│   ├── config.example.toml    # Annotated configuration template
│   ├── config.toml            # Your local config (gitignored)
│   ├── .env                   # Secret env vars (gitignored)
│   └── certs/                 # CA certificates for TLS (gitignored)
├── Dockerfile                 # Multi-stage Alpine build
└── docker-compose.yml         # Local dev setup
```

---

## Layer architecture

```
┌─────────────────────────────────────┐
│         LLM Client (MCP)            │
├─────────────────────────────────────┤
│  opengrok-mcp (binary)              │
│  ├── transport/      stdio / HTTP   │
│  ├── mcp/tools.rs    tool schemas   │
│  ├── mcp/mod.rs      tool handlers  │
│  ├── config.rs       config load    │
│  └── health.rs       health/metrics │
├─────────────────────────────────────┤
│  opengrok-core (library)            │
│  ├── application.rs  service layer  │
│  ├── domain.rs       data models    │
│  └── infrastructure/                │
│      ├── client.rs   HTTP client    │
│      ├── tls.rs      TLS setup      │
│      ├── cache.rs    response cache │
│      ├── rate_limit  rate limiter   │
│      └── format.rs   HTML stripping │
├─────────────────────────────────────┤
│         OpenGrok API (REST)         │
└─────────────────────────────────────┘
```

### Dependency direction

`opengrok-mcp` depends on `opengrok-core`. `opengrok-core` has **no MCP
dependencies** — it's a pure HTTP client library that can be reused in
other contexts.

---

## Crate responsibilities

### `opengrok-core` — domain & infrastructure

| Module | Lines | Purpose |
|---|---|---|
| `domain.rs` | 1206 | All data types: `SearchResult`, `FileContent`, `HistoryEntry`, `Project`, `DirectoryEntry`, error types (`CoreError`) |
| `application.rs` | 480 | High-level operations: `search()`, `get_file_content()`, `get_history()`, with pagination, caching, and formatting |
| `infrastructure/client.rs` | 930 | `reqwest`-based HTTP client: request building, auth header injection, response parsing, OpenGrok quirk handling |
| `infrastructure/tls.rs` | 476 | TLS configuration: custom CA loading, rustls setup, PEM parsing |
| `infrastructure/format.rs` | 479 | HTML tag stripping (`<b>`, `<i>`, etc.), result text normalisation |
| `infrastructure/cache.rs` | 221 | In-memory cache with TTL eviction using `DashMap` |
| `infrastructure/rate_limit.rs` | 110 | Token-bucket rate limiter via `governor` |

### `opengrok-mcp` — MCP server

| Module | Lines | Purpose |
|---|---|---|
| `mcp/mod.rs` | 485 | MCP server initialisation, 25 tool handler dispatch, error mapping (`CoreError` → MCP error codes) |
| `mcp/tools.rs` | 230 | Tool type definitions with JSON Schema (schemars): names, descriptions, parameter types, defaults (25 tools) |
| `config.rs` | 466 | Config loading: TOML parsing, env var overrides, validation |
| `transport/http.rs` | 67 | Axum router with `NeverSessionManager` (stateless, MCP 2026-07-28 protocol): MCP endpoint, health, readiness, metrics |
| `transport/stdio.rs` | 20 | stdin/stdout transport via rmcp |
| `health.rs` | 165 | Health check handlers: liveness, readiness with OpenGrok probe, Prometheus metrics collection |
| `main.rs` | 124 | Entry point: CLI parsing, config init, transport selection, shutdown signal handling |

---

## Data flow

```
LLM Client
  │
  │  MCP request: { tool: "search", params: { query: "...", project: "aosp" } }
  ▼
transport/stdio.rs or http.rs   ← receives MCP message
  │
  ▼
mcp/mod.rs                       ← routes by tool name
  │
  ▼
opengrok-core::application.rs    ← service logic, cache check, rate limit
  │
  ▼
opengrok-core::infrastructure/
  ├── cache.rs                   ← return cached if hit
  ├── rate_limit.rs              ← wait if throttled
  ├── client.rs                  ← HTTP request to OpenGrok
  │     │
  │     ▼
  │   tls.rs                     ← TLS with custom CA (if configured)
  │     │
  │     ▼
  │   OpenGrok API
  │
  ▼
opengrok-core::infrastructure/
  └── format.rs                  ← strip HTML, normalise result
  │
  ▼
application.rs                   ← paginate, build response with has_more
  │
  ▼
mcp/mod.rs                       ← serialize to MCP response
  │
  ▼
transport/                       ← send response back to LLM
  │
  ▼
LLM Client
```

---

## Design decisions

### Why two crates?

The split between `opengrok-core` (library) and `opengrok-mcp` (binary) keeps MCP
dependencies out of the core HTTP client. This means:

- The core can be used in non-MCP contexts (e.g., a CLI tool or web UI)
- Compile times are faster when working on the core
- Dependencies are clearly separated — `rmcp`, `axum`, `schemars` only appear in the binary

### Why reqwest + rustls?

- `reqwest` is the de-facto Rust HTTP client — well-tested, async, supports TLS
- `rustls` is a pure-Rust TLS implementation — avoids OpenSSL linkage issues, especially in
  Docker Alpine builds
- `rustls-native-certs` provides integration with the system trust store when needed

### Why DashMap for cache?

`DashMap` is a concurrent hashmap — it allows lock-free reads and fine-grained
locking for writes. For an MCP server handling parallel LLM requests, this avoids
contention that a `Mutex<RwLock<HashMap>>` would create.

### Why governor for rate limiting?

`governor` implements the Generic Cell Rate Algorithm (GCRA) — a token-bucket
variant. It's lightweight, async-compatible, and well-suited for protecting a
single backend (OpenGrok) from excessive request rates.
