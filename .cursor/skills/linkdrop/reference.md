# linkdrop reference

## Environment variables (server)

| Variable | Default | Description |
|----------|---------|-------------|
| `LINKDROP_TOKEN` | — | Required API auth token |
| `LINKDROP_URL` | `http://localhost:8080` | Public base URL for generated links |
| `LINKDROP_DATA_DIR` | `./data` | SQLite + HTML storage |
| `PORT` | `8080` | Listen port |

## Environment variables (CLI)

| Variable | Description |
|----------|-------------|
| `LINKDROP_URL` | Server base URL |
| `LINKDROP_TOKEN` | API token |
| `LINKDROP_CONFIG` | Override config file path |

Config file default: `~/.config/linkdrop/config.toml`. Env vars override file values.

## API

Auth: `Authorization: Bearer <token>` or `X-Linkdrop-Token: <token>`

### POST /api/pages

```json
{
  "html": "<html>...</html>",
  "slug": "optional-slug",
  "force": false,
  "ttl": "24h"
}
```

Response:

```json
{
  "slug": "my-preview",
  "url": "https://link.ddroid.dev/my-preview",
  "created_at": "2026-06-25T12:00:00Z",
  "expires_at": null,
  "size_bytes": 1234
}
```

### GET /api/pages

Returns array of page objects (same shape as POST response).

### DELETE /api/pages/{slug}

Returns 204 on success.

### GET /{slug}

Serves HTML (no auth).

## Dokploy

1. New application → Docker / Dockerfile from this repo
2. Domain: `link.ddroid.dev`
3. Port: `8080`
4. Volume: mount `/data`
5. Env: `LINKDROP_TOKEN`, `LINKDROP_URL=https://link.ddroid.dev`

## TTL format

Examples: `3600`, `24h`, `7d`, `30m`

Expired pages are removed by an internal 24-hour sweeper.
