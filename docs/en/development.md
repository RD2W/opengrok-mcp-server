# Development

Previous: [← Architecture](./architecture.md)

---

## Getting started

```bash
git clone <repo-url> opengrok-mcp-server
cd opengrok-mcp-server
cargo build --workspace
cargo test --workspace
```

Development happens on the `dev` branch. Cut feature branches from it:

```bash
git checkout dev
git checkout -b feat/my-feature
```

---

## Running tests

```bash
# All tests (102 at time of writing)
cargo test --workspace

# Specific crate
cargo test -p opengrok-core
cargo test -p opengrok-mcp

# With output
cargo test -- --nocapture

# Run ignored (integration) tests
cargo test -- --ignored
```

---

## CI pipeline

CI runs on every push to `dev`, `main`, and `ci` branches, and on all PRs:

| Job | Command | Purpose |
|---|---|---|
| Format | `cargo fmt --all -- --check` | Ensures consistent code style |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Catches common mistakes and style issues |
| Tests | `cargo test --workspace --locked` | Runs all unit and integration tests |
| Build | `cargo build --workspace --locked --release` | Verifies the release build compiles |

GitHub Actions workflow: `.github/workflows/ci.yml`

---

## Code conventions

### General

- **Language:** English comments and commit messages
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/) —
  `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`
- **Formatting:** `rustfmt` with default settings
- **Linting:** `clippy` with `-D warnings` — all warnings are errors in CI

### SPDX headers

Every new `.rs` file must start with:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

See any existing source file for the exact format.

### Module organisation

- Small, focused files with clear responsibilities
- One major type or concern per file
- `mod.rs` files for module declarations and re-exports only

---

## Adding a new MCP tool

1. **Add the domain type** in `opengrok-core/src/domain.rs` if the API returns
   a new response shape.

2. **Add the client method** in `opengrok-core/src/infrastructure/client.rs` —
   implement the HTTP call to the OpenGrok REST API endpoint.

3. **Add the application method** in `opengrok-core/src/application.rs` —
   wire up caching, rate limiting, and formatting.

4. **Define the tool schema** in `opengrok-mcp/src/mcp/tools.rs`:
   ```rust
   #[tool(description = "Search for a symbol definition across the codebase")]
   async fn search_define(
       symbol: String,
       #[param(description = "Project name to limit search scope")]
       project: Option<String>,
   ) -> Result<CallToolResult, McpError> {
       // …
   }
   ```
   Use `#[param(description = "...")]` for every parameter — these descriptions
   are exposed to LLM clients and directly affect tool call quality.

5. **Register the handler** in `opengrok-mcp/src/mcp/mod.rs` — add the tool
   to the server's tool list and map it to the application method.

6. **Add tests** — unit tests for the domain type, integration tests for the
   HTTP client (mock the OpenGrok response), and handler tests for the MCP layer.

---

## Documentation

When behaviour changes, update:

- The relevant `docs/en/` and `docs/ru/` pages (keep them in sync)
- `README.md` if it affects the quick start or feature list
- `CHANGELOG.md` — add an entry under `[Unreleased]`
- `config/config.example.toml` — if configuration options change

---

## Pre-PR checklist

Before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

All four must pass. CI enforces the first three — the build check catches
compilation issues that tests might miss.

---

## Release workflow

Releases are automated via `.github/workflows/release.yml`:

1. Push a tag like `v0.1.0`
2. CI builds multi-arch Docker images and creates a GitHub Release
3. Binary artifacts are attached to the release

Manual release steps (for debugging the workflow):

```bash
docker build -t opengrok-mcp:v0.1.0 .
docker tag opengrok-mcp:v0.1.0 opengrok-mcp:latest
```
