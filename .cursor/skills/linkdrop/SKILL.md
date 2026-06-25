---
name: linkdrop
description: Push HTML files to linkdrop and get shareable URLs at link.ddroid.dev. Use when publishing HTML previews, sharing agent-generated pages, deploying static HTML for review, or when the user mentions linkdrop, link.ddroid.dev, or pushing HTML to a URL.
---

# linkdrop

Push single HTML files to `https://link.ddroid.dev/<slug>` for browser viewing.

## Prerequisites

One-time setup (config file or env — not per command):

```toml
# ~/.config/linkdrop/config.toml
url = "https://link.ddroid.dev"
token = "your-token"
```

Or:

```bash
export LINKDROP_URL=https://link.ddroid.dev
export LINKDROP_TOKEN=your-token
```

CLI must be on PATH (`linkdrop`). Build from this repo: `cargo build --release` → `target/release/linkdrop`.

## Commands

```bash
# Random ID URL
linkdrop push path/to/file.html

# Readable slug
linkdrop push path/to/file.html --slug my-preview

# From generated HTML (stdin)
echo '<html>...</html>' | linkdrop push --stdin --slug quick-demo

# Overwrite existing slug
linkdrop push file.html --slug my-preview --force

# Expire after TTL (optional)
linkdrop push file.html --slug temp --ttl 24h

linkdrop list
linkdrop delete my-preview
```

`push` prints the public URL on success. Share that URL with the user.

## Slug rules

- Optional; if omitted, server assigns a random ID
- Allowed: `a-z`, `0-9`, `-` only; 1–64 characters; no leading/trailing hyphen
- Collision without `--force` → error (409)

## Limits

- Single HTML file only (inline CSS/JS OK; no asset folders)
- Max 5 MB per upload
- Root `https://link.ddroid.dev/` is a built-in Flappy Bird page — not user-uploadable

## Error handling

| Situation | Action |
|-----------|--------|
| 409 slug exists | Use `--force` or pick a new slug |
| 413 too large | Shrink HTML or remove inline assets |
| 401 unauthorized | Check `LINKDROP_TOKEN` |
| 404 on view | Slug wrong, deleted, or TTL expired |

## Workflow for agents

1. Write HTML to a file (or pipe to `--stdin`)
2. `linkdrop push ...` with a descriptive `--slug` when the user needs a memorable URL
3. Return the printed URL to the user
4. `linkdrop delete <slug>` when cleaning up temporary previews

## Additional resources

- API and Dokploy deploy details: [reference.md](reference.md)
