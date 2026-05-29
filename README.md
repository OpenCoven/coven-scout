# coven-scout

> A fast, safe MCP server for filesystem and web operations — built for the Coven ecosystem

`coven-scout` is a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server written in Rust that gives AI agents sandboxed, structured access to the local filesystem and web resources. It ships as a **single static binary** with no runtime dependencies.

---

## Features

- 🗂 **Filesystem operations** — read, write, list, find, copy, move, delete, archive
- 🌐 **Web fetching** — fetch URLs, convert HTML → Markdown
- 🔒 **Security sandbox** — configurable path allowlist; all ops validated against it
- ✅ **Checksums** — SHA-256, SHA-512, MD5
- 🔍 **Diffs** — unified diffs between two files
- 📦 **Archives** — create/extract zip and tar.gz
- 🖼 **Metadata** — MIME type, size, timestamps, permissions
- 🔎 **Find** — search by name, content, extension, size, date, MIME type
- ⚡️ **Single binary** — no Node.js/npm required
- 🦀 **Memory-safe** — Rust's ownership model prevents common vulnerabilities
- 📊 **Structured logging** — per-operation spans via `tracing`

---

## Installation

### From source

```bash
git clone https://github.com/OpenCoven/coven-scout
cd coven-scout
cargo build --release
# Binary at: target/release/coven-scout
```

### Via cargo install

```bash
cargo install --git https://github.com/OpenCoven/coven-scout
```

---

## Configuration

All configuration is via environment variables:

| Variable | Default | Description |
|---|---|---|
| `COVEN_SCOUT_ALLOWED_PATHS` | `~:/tmp` | Colon-separated allowed path prefixes |
| `COVEN_SCOUT_LOG_LEVEL` | `INFO` | TRACE/DEBUG/INFO/WARN/ERROR |
| `COVEN_SCOUT_LOG_FILE` | (stderr) | Path to log file, or `NONE` |
| `COVEN_SCOUT_MAX_FILE_READ_BYTES` | `52428800` (50MB) | Max bytes per file read |
| `COVEN_SCOUT_MAX_URL_DOWNLOAD_BYTES` | `20971520` (20MB) | Max URL download |
| `COVEN_SCOUT_HTTP_TIMEOUT_MS` | `30000` | HTTP request timeout ms |
| `COVEN_SCOUT_MAX_RECURSIVE_DEPTH` | `10` | Max recursion depth |
| `COVEN_SCOUT_MAX_FIND_RESULTS` | `1000` | Max results from find |
| `COVEN_SCOUT_DEFAULT_CHECKSUM` | `sha256` | Default checksum algorithm |

---

## MCP Client Configuration

Add this to your MCP client config (e.g. Claude Desktop `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "coven-scout": {
      "command": "/path/to/coven-scout",
      "env": {
        "COVEN_SCOUT_ALLOWED_PATHS": "/Users/you/projects:/tmp"
      }
    }
  }
}
```

---

## Tool Reference

### `read`

| Operation | Description |
|---|---|
| `content` | Read file(s) or URL(s); supports `text`, `base64`, `markdown`, `checksum` formats |
| `metadata` | File/URL info: size, MIME, timestamps, permissions |
| `diff` | Unified diff between exactly 2 files |

### `write`

| Operation | Description |
|---|---|
| `put` | Write a file (`overwrite`/`append`/`error_if_exists`) |
| `mkdir` | Create directory |
| `copy` | Copy file or directory |
| `move` | Move/rename |
| `delete` | Delete file or directory |
| `touch` | Create empty file or update mtime |
| `archive` | Create `zip` or `tar.gz` archive |
| `unarchive` | Extract archive |

### `list`

| Operation | Description |
|---|---|
| `entries` | List directory with optional recursion |
| `system_info` | Server capabilities or filesystem stats |

### `find`

| Operation | Description |
|---|---|
| `search` | Find files by name, content, extension, size, date, MIME type |

### `test`

| Operation | Description |
|---|---|
| `echo` | Echo back parameters |
| `generate_error` | Return a test error |

---

## Security

coven-scout enforces a path allowlist on every filesystem operation:

- Default allowed: `~` (home dir) and `/tmp`
- Set `COVEN_SCOUT_ALLOWED_PATHS` to restrict or expand access
- All paths are resolved to absolute form before validation
- Path traversal attempts (`../`) are rejected after normalization

---

## Contributing

> ⚠️ **Contribution Freeze (until July 2026)**
> We are currently only accepting **Issues and Bug Reports**.
> Pull Requests will not be reviewed or merged until July 2026.
> Please open an issue to discuss your idea — we'll revisit PRs after the freeze lifts.

---

## License

MIT — see [LICENSE](LICENSE)
