use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info, warn};
use crate::config::Config;
use crate::protocol::{Request, Response, ToolResult};
use crate::tools;

/// Run the MCP server loop: read JSON-RPC from stdin, write responses to stdout.
pub async fn run(config: Arc<Config>) -> anyhow::Result<()> {
    info!("coven-scout MCP server starting");

    if config.is_using_defaults() {
        eprintln!(
            "╔══════════════════════════════════════════════════════════╗\n\
             ║  coven-scout  —  Coven Ecosystem MCP Server v{}{}  ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  ⚠️  Using default allowed paths: ~ and /tmp             ║\n\
             ║  Set COVEN_SCOUT_ALLOWED_PATHS to restrict access        ║\n\
             ╚══════════════════════════════════════════════════════════╝",
            env!("CARGO_PKG_VERSION"),
            " ".repeat(20usize.saturating_sub(env!("CARGO_PKG_VERSION").len()))
        );
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::BufWriter::new(stdout);

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        debug!("Received: {line}");

        let response = handle_line(&line, Arc::clone(&config)).await;

        if let Some(resp) = response {
            let json = serde_json::to_string(&resp)?;
            debug!("Sending: {json}");
            stdout.write_all(json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    info!("coven-scout MCP server shutting down");
    Ok(())
}

async fn handle_line(line: &str, config: Arc<Config>) -> Option<Response> {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to parse JSON-RPC request: {e}");
            return Some(Response::error(None, -32700, format!("Parse error: {e}")));
        }
    };

    // Notifications have no id — process but don't respond
    let is_notification = req.id.is_none();

    let id = req.id.clone();
    let method = req.method.as_str();

    match method {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "coven-scout",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(Response::success(id, result))
        }

        "initialized" => {
            // Notification — no response
            info!("Client initialized");
            None
        }

        "tools/list" => {
            let tool_list = tools::tool_list();
            let result = serde_json::json!({ "tools": tool_list });
            Some(Response::success(id, result))
        }

        "tools/call" => {
            let name = req.params.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = req.params.get("arguments")
                .cloned()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            let tool_result = tools::dispatch(&name, arguments, config).await
                .unwrap_or_else(|e| ToolResult::err(e.to_string()));

            let result = serde_json::to_value(&tool_result).unwrap_or(serde_json::Value::Null);
            Some(Response::success(id, result))
        }

        "ping" => {
            Some(Response::success(id, serde_json::json!({})))
        }

        other => {
            warn!("Unknown method: {other}");
            if is_notification {
                None
            } else {
                Some(Response::error(id, -32601, format!("Method not found: {other}")))
            }
        }
    }
}
