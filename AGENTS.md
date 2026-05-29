# AGENTS.md — coven-scout

coven-scout is a **Rust MCP server** that gives AI agents sandboxed access to the local filesystem and the web. It speaks JSON-RPC 2.0 over stdin/stdout and implements the [Model Context Protocol](https://modelcontextprotocol.io).

## What this codebase is

| Layer | Path | What it does |
|---|---|---|
| Entry point | `src/main.rs` | Loads config, starts the MCP server loop |
| Server loop | `src/server.rs` | Reads newline-delimited JSON-RPC from stdin, dispatches tools, writes responses to stdout |
| Protocol types | `src/protocol.rs` | `Request`, `Response`, `Tool`, `ToolResult`, `Content` — strict MCP types |
| Config | `src/config.rs` | All env var config (`COVEN_SCOUT_*`); see README for full list |
| Security | `src/security.rs` | Path validation — resolves symlinks, enforces `COVEN_SCOUT_ALLOWED_PATHS` |
| Error types | `src/error.rs` | `CovenScoutError` via `thiserror` |
| Tools | `src/tools/` | One file per tool: `read`, `write`, `list`, `find`, `test` |
| Utilities | `src/utils/` | `web` (HTTP + HTML→Markdown), `archive` (zip/tar.gz), `checksum`, `diff` |
| Docs | `docs/spec.md` | Full JSON Schema for all tool inputs/outputs |

## How to work on this

```bash
cargo check          # fast type + borrow check
cargo build          # full build
cargo test           # run tests
cargo clippy         # lints
cargo fmt            # format
```

## Key conventions

- **No panics in library code.** Use `?` and `Result`. `unwrap()` only in tests or where logically impossible.
- **All FS operations go through `security::validate_path`** — never bypass it.
- **Log to stderr only** — stdout is the MCP wire protocol. Mixing them breaks the server.
- **Structured tracing** — use `tracing::{info, warn, error, debug}` with span context, not `println!`.
- **Tool handlers** receive `Arc<Config>` + raw `serde_json::Value` args. Return `ToolResult`.

## Adding a new tool

1. Add a new file in `src/tools/your_tool.rs`
2. Implement `pub async fn handle(args: serde_json::Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError>`
3. Register the tool definition in `src/tools/mod.rs` → `all_tools()` list
4. Add dispatch arm in `src/tools/mod.rs` → `dispatch()`
5. Document the JSON Schema in `docs/spec.md`

## Security model

- Allowed paths controlled by `COVEN_SCOUT_ALLOWED_PATHS` (colon-separated, tilde-expanded)
- Symlinks are resolved before validation — no escape via symlink chains
- Default allowed: `~:/tmp` — first run will emit a warning if using defaults
- All HTTP requests are capped by `COVEN_SCOUT_MAX_URL_DOWNLOAD_BYTES` and `COVEN_SCOUT_HTTP_TIMEOUT_MS`

## Pre-commit hooks

This repo uses [pre-commit](https://pre-commit.com) with `gitleaks` for secret scanning.

```bash
pre-commit install          # install hooks (one-time)
pre-commit run --all-files  # run manually
```

**Never commit secrets.** The `COVEN_SCOUT_*` env vars are config — not hardcoded values.

## Coven ecosystem context

coven-scout is part of [OpenCoven](https://github.com/OpenCoven). It is designed to be embedded as an MCP server in:
- **CastCodes** — the canonical OpenCoven application
- **Coven agents** — any agent needing filesystem/web tool access
- **Claude Desktop** or any MCP-compatible client

Point your MCP client at the `coven-scout` binary. That's it.

## Related

- [OpenCoven org](https://github.com/OpenCoven)
- [MCP spec](https://modelcontextprotocol.io)
- [coven-scout README](./README.md)
- [Tool spec](./docs/spec.md)
