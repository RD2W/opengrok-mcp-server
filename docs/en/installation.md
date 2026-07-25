# Installation

Next: [Usage →](./usage.md)
Previous: [← Overview](./overview.md)

---

## Requirements

- **Rust** 1.97 or later (edition 2024)
- **OpenGrok** instance accessible over HTTP(S)
- **Docker** (optional — for containerised deployment)

---

## Building from source

```bash
git clone <repo-url> opengrok-mcp-server
cd opengrok-mcp-server

# Build in release mode
cargo build --release

# The binary is at:
#   target/release/opengrok-mcp
```

### Configuration

```bash
cp config/config.example.toml config/config.toml
```

Edit `config/config.toml` — at minimum, set:

```toml
[opengrok]
base_url = "https://your-opengrok.example.com"

[opengrok.auth]
mode = "token"   # or "basic" / "none"
token_env = "OPENGROK_TOKEN"
```

Set credentials via environment variables:

```bash
export OPENGROK_TOKEN="your-token-here"
# or for Basic auth:
export OPENGROK_USERNAME="user"
export OPENGROK_PASSWORD="pass"
```

### Run

```bash
cargo run --release
```

The server starts in stdio mode by default, ready for MCP clients.

---

## Docker

### Local development

```bash
# Set credentials in config/.env:
#   OPENGROK_TOKEN=your-token
#   OPENGROK_URL=https://opengrok.example.com

docker compose up -d
```

### Docker Hub (pre-built image)

Pre-built multi-arch images (linux/amd64, linux/arm64) are published to
[Docker Hub](https://hub.docker.com/r/rd2w/opengrok-mcp/tags) on every
tagged release.

```bash
# Pull the latest release
docker pull rd2w/opengrok-mcp:latest

# Or a specific version
docker pull rd2w/opengrok-mcp:v0.1.0

# Use the docker-compose file for pre-built images
docker compose -f docker-compose.hub.yml up -d
```

The `docker-compose.hub.yml` is identical to `docker-compose.yml` except it
uses `image:` instead of `build:` — no Rust toolchain or compilation required
on the target host.

### Remote / air-gapped deployment

For hosts **without internet access** (common in corporate environments), build
the image on a connected machine, then transfer it as a self-contained archive.
The multi-stage Docker build bakes all dependencies into the image — no network
access is required at runtime.

```bash
# 1. Build on a machine with internet access
#    (pulls base images, fetches Rust crates, compiles — all baked in)
docker build -t opengrok-mcp:latest .

# 2. Export as a single portable archive (~35 MB)
docker save opengrok-mcp:latest | gzip > opengrok-mcp.tar.gz

# 3. Transfer to the air-gapped host (USB drive, scp to jump host, etc.)
scp opengrok-mcp.tar.gz docker-compose.yml remote-host:~/mcp/

# 4. On the remote host — load and run (no internet needed)
ssh remote-host
cd ~/mcp/
docker load < opengrok-mcp.tar.gz               # imports the image

# Prepare configuration
mkdir -p config
cp /path/to/config.toml config/                 # your config
cp /path/to/your-ca.crt config/certs/           # CA cert (if using custom TLS)

# Create config/.env with secrets (never commit this file)
echo 'OPENGROK_TOKEN=your-token' > config/.env
echo 'OPENGROK_URL=https://opengrok.example.com' >> config/.env

docker compose up -d
```

> **Air-gapped checklist:** The image includes the Alpine base, `ca-certificates`,
> the compiled binary, and all Rust dependencies. The only external dependency is
> the OpenGrok instance itself — the MCP server makes **outbound** HTTPS requests
> to it, so the host needs network access to OpenGrok (but not to the internet
> at large).

### Image size

The multi-stage Docker build produces an Alpine-based image of approximately
**35 MB** — small enough for easy transfer over slow connections.

---

## TLS with custom CAs

If your OpenGrok instance uses a corporate or self-signed certificate:

1. Place the CA certificate PEM file at `config/certs/`:
   ```bash
   cp your-ca.crt config/certs/
   ```

2. Configure in `config.toml`:
   ```toml
   [opengrok]
   ca_cert = "./config/certs/your-ca.crt"
   ```

3. Or use environment variables:
   ```bash
   export OPENGROK_CA_CERT=/path/to/ca.pem
   export SSL_CERT_FILE=/path/to/ca.pem
   export SSL_CERT_DIR=/path/to/certs/
   ```

For Docker, the `config/` directory is mounted read-only — certificates are
picked up automatically.

### Disabling TLS verification (insecure!)

Only for trusted internal networks:

```toml
[opengrok]
verify_ssl = false
```

Or `export OPENGROK_VERIFY_SSL=false`.
