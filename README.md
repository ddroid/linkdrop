# linkdrop

Push HTML files and get a shareable URL at `https://sub.domain.com/<slug>`.

## Components

- **linkdrop-server** — HTTP API + static HTML serving (Flappy Bird at `/`)
- **linkdrop** CLI — `push`, `delete`, `list` for agents and humans

## One-time setup

### Server (Dokploy)

| Variable | Example | Required |
|----------|---------|----------|
| `LINKDROP_TOKEN` | random secret | yes |
| `LINKDROP_URL` | `https://sub.domain.com` | yes |
| `LINKDROP_DATA_DIR` | `/data` | no (default `/data` in Docker) |
| `PORT` | `8080` | no |

Mount a persistent volume at `/data`.

**Dokploy:** deploy from this repo's `Dockerfile`, set domain `sub.domain.com`, expose port 8080, mount volume `/data`.

### CLI (agents / local)

Create `~/.config/linkdrop/config.toml`:

```toml
url = "https://sub.domain.com"
token = "your-token-here"
```

Or set environment variables (they override the config file):

```bash
export LINKDROP_URL=https://sub.domain.com
export LINKDROP_TOKEN=your-token-here
```

## Install the CLI

### Option A — prebuilt binary (no Rust toolchain needed)

One-liner; downloads the right binary for your platform from GitHub Releases
and installs it to `/usr/bin/linkdrop`:

```bash
curl -fsSL https://raw.githubusercontent.com/ddroid/linkdrop/main/install.sh | sh
```

Install elsewhere or pin a version:

```bash
# custom install dir
curl -fsSL https://raw.githubusercontent.com/ddroid/linkdrop/main/install.sh | sh -s -- --bindir /usr/local/bin

# specific release tag
curl -fsSL https://raw.githubusercontent.com/ddroid/linkdrop/main/install.sh | sh -s -- --tag v0.1.0
```

Prebuilt binaries are published for:

| Platform | Asset |
|----------|-------|
| Linux x86_64 | `linkdrop-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `linkdrop-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `linkdrop-aarch64-apple-darwin.tar.gz` |

The Linux binaries use `rustls-tls` and depend only on glibc (no OpenSSL), so
they run on most modern glibc-based distros. Intel Macs are not built
explicitly — Apple Silicon Macs can run x86_64 builds via Rosetta 2 if needed.
Downloads are verified against a published `sha256sums.txt`.

Uninstall a prebuilt install: `sudo rm /usr/bin/linkdrop`.

### Option B — build from source

From the repo root:

```bash
# Build the release binary as your normal user (do NOT use sudo here —
# rustup toolchains live in your home dir and sudo runs as root)
make build

# Install to /usr/bin/linkdrop — only this step needs root
sudo make install

# Uninstall
sudo make uninstall
```

Install prefix is configurable:

```bash
make install PREFIX=/opt/linkdrop
```

This uses standard `PREFIX`/`BINDIR`/`DESTDIR` variables, so packagers can
override them. After install, `linkdrop` is on `PATH` and available as
`/usr/bin/linkdrop`.

### Releasing new binaries

Maintainers: push a tag to trigger the release workflow, which builds all
platforms and publishes a GitHub Release with tarballs + checksums.

```bash
git tag v0.1.0
git push origin v0.1.0
```

## CLI usage

```bash
# Push a file (random ID in URL)
linkdrop push report.html

# Push with a readable slug
linkdrop push report.html --slug my-report

# Push from stdin
echo '<h1>hi</h1>' | linkdrop push --stdin --slug quick-preview

# Overwrite existing slug
linkdrop push report.html --slug my-report --force

# Temporary preview (24h TTL)
linkdrop push report.html --slug temp-demo --ttl 24h

# List / delete
linkdrop list
linkdrop delete my-report
```

`push` prints the public URL on success.

## Limits

- Single HTML file per push (max 5 MB)
- Slugs: `a-z`, `0-9`, `-` only; 1–64 chars; no leading/trailing hyphen
- Default slug collision → error; use `--force` to overwrite
- Expired pages removed by a daily sweeper

## Development

```bash
cargo build --release
LINKDROP_TOKEN=test LINKDROP_URL=http://localhost:8080 LINKDROP_DATA_DIR=./data \
  cargo run -p linkdrop-server
```

## API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/` | no | Flappy Bird landing |
| `GET` | `/{slug}` | no | Serve HTML |
| `POST` | `/api/pages` | Bearer token | Upload HTML JSON body |
| `GET` | `/api/pages` | Bearer token | List pages |
| `DELETE` | `/api/pages/{slug}` | Bearer token | Delete page |
