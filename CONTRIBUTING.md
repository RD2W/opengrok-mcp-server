# Contributing to opengrok-mcp

Thanks for your interest! Contributions are welcome.

## Getting started

```bash
git clone <repo> && cd opengrok-mcp-server
cargo build --workspace
cargo test --workspace
```

Development happens on the `dev` branch; cut feature branches from it (`feat/...`, `fix/...`).

## Before you open a PR

All of these must pass (CI enforces them):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Conventions

- **English** code comments and commit messages; Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `test:`, `refactor:`).
- New source files start with the SPDX header (see any existing `.rs` file).
- Prefer small, focused files with clear responsibilities.

## MCP protocol changes

The MCP protocol is standardized. When adding or changing MCP behavior:

- Reference the [MCP specification](https://spec.modelcontextprotocol.io/).
- Add tests for new tool handlers and transport layers.

## Documentation

Update docs when behavior changes, and add an entry to `CHANGELOG.md` under "Unreleased".

## License

By contributing you agree that your contributions are licensed under **GPL-3.0-or-later**, consistent with the rest of the project.
