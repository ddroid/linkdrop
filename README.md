# linkdrop

Push HTML files and get a shareable URL at `https://link.ddroid.dev/<slug>`.

## Components

- **linkdrop-server** — HTTP API + static HTML serving (Flappy Bird at `/`)
- **linkdrop** CLI — `push`, `delete`, `list` for agents and humans

## One-time setup

### Server (Dokploy)

| Variable | Example | Required |
|----------|---------|----------|
| `LINKDROP_TOKEN` | random secret | yes |
| `LINKDROP_URL` | `https://link.ddroid.dev` | yes |
| `LINKDROP_DATA_DIR` | `/data` | no (default `/data` in Docker) |
| `PORT` | `8080` | no |

Mount a persistent volume at `/data`.

**Dokploy:** deploy from this repo's `Dockerfile`, set domain `link.ddroid.dev`, expose port 8080, mount volume `/data`.

### CLI (agents / local)

Create `~/.config/linkdrop/config.toml`:

```toml
url = "https://link.ddroid.dev"
token = "your-token-here"
```

Or set environment variables (they override the config file):

```bash
export LINKDROP_URL=https://link.ddroid.dev
export LINKDROP_TOKEN=your-token-here
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
