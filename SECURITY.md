# Security Policy

## Scope

`opengrok-mcp` is an MCP server that proxies search requests to an OpenGrok instance. The main attack surfaces are:

- **Network exposure** when running in Streamable HTTP mode — the HTTP server is accessible over the network.
- **OpenGrok query injection** — maliciously crafted search parameters could expose or enumerate source code.
- **Credential leakage** — API tokens, OpenGrok URLs, or other secrets in configuration, logs, or environment variables.

There is no filesystem write surface — the server only reads from the configured OpenGrok instance and serves results over MCP.

## Reporting a vulnerability

Please report security issues **privately**, not via public issues:

- Use GitHub's "Report a vulnerability" (Security → Advisories), or
- email the maintainer: `mkrutovercev@yandex.ru`.

Include steps to reproduce and the impact. We'll acknowledge and work on a fix; a coordinated disclosure timeline can be agreed if needed.

## Supported versions

This is pre-1.0 software; fixes land on the latest `dev`/release branch. Pin a released tag for reproducible builds (`Cargo.lock` is committed).
