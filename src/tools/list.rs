use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{Config, CovenScoutError};
use crate::protocol::{Tool, ToolResult};
use crate::security::validate_path;
use tracing::instrument;

pub fn tool_descriptor() -> Tool {
    Tool {
        name: "list".to_string(),
        description: "List directory contents or get system/server information.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["entries", "system_info"],
                    "description": "Operation to perform"
                },
                "path": { "type": "string", "description": "Directory to list (for entries)" },
                "recursive_depth": { "type": "integer", "description": "Recursion depth (0 = top-level only)" },
                "calculate_recursive_size": { "type": "boolean", "description": "Calculate total size for directories" },
                "info_type": {
                    "type": "string",
                    "enum": ["server_capabilities", "filesystem_stats"],
                    "description": "Type of system info (for system_info operation)"
                }
            },
            "required": ["operation"]
        }),
    }
}

#[derive(Debug, Deserialize)]
struct ListArgs {
    operation: String,
    path: Option<String>,
    recursive_depth: Option<u32>,
    calculate_recursive_size: Option<bool>,
    info_type: Option<String>,
}

#[instrument(skip(config))]
pub async fn handle(args: Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let args: ListArgs = serde_json::from_value(args)
        .map_err(|e| CovenScoutError::InvalidArgument(e.to_string()))?;

    match args.operation.as_str() {
        "entries" => handle_entries(args, config).await,
        "system_info" => handle_system_info(args, config).await,
        other => Ok(ToolResult::err(format!("Unknown list operation: {other}"))),
    }
}

async fn handle_entries(args: ListArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let path_str = args.path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("entries requires 'path'".to_string()))?;
    let resolved = validate_path(path_str, &config)?;
    let max_depth = args.recursive_depth.unwrap_or(1).min(config.max_recursive_depth);
    let calc_size = args.calculate_recursive_size.unwrap_or(false);

    let mut entries = Vec::new();
    collect_entries(&resolved, &resolved, 0, max_depth, calc_size, &mut entries)?;

    let result = json!({
        "path": resolved.display().to_string(),
        "entries": entries,
    });

    Ok(ToolResult::ok(serde_json::to_string_pretty(&result).unwrap_or_default()))
}

fn collect_entries(
    base: &std::path::Path,
    dir: &std::path::Path,
    current_depth: u32,
    max_depth: u32,
    calc_size: bool,
    entries: &mut Vec<Value>,
) -> Result<(), CovenScoutError> {
    let read_dir = std::fs::read_dir(dir)?;
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        let relative = path.strip_prefix(base).unwrap_or(&path);
        let entry_type = if meta.is_dir() { "directory" } else if meta.is_symlink() { "symlink" } else { "file" };
        let mime = if meta.is_file() {
            mime_guess::from_path(&path).first().map(|m| m.to_string()).unwrap_or_else(|| "application/octet-stream".to_string())
        } else {
            "inode/directory".to_string()
        };

        let size = if meta.is_dir() && calc_size {
            compute_dir_size(&path)
        } else {
            meta.len()
        };

        let modified = meta.modified().ok().map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });

        let created = meta.created().ok().map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });

        entries.push(json!({
            "name": entry.file_name().to_string_lossy(),
            "path": path.display().to_string(),
            "relative_path": relative.display().to_string(),
            "type": entry_type,
            "size": size,
            "mime_type": mime,
            "modified": modified,
            "created": created,
        }));

        if meta.is_dir() && current_depth < max_depth {
            collect_entries(base, &path, current_depth + 1, max_depth, calc_size, entries)?;
        }
    }
    Ok(())
}

fn compute_dir_size(path: &std::path::Path) -> u64 {
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

async fn handle_system_info(args: ListArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let info_type = args.info_type.as_deref().unwrap_or("server_capabilities");

    let result = match info_type {
        "server_capabilities" => json!({
            "server": "coven-scout",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol_version": "2024-11-05",
            "capabilities": {
                "tools": ["read", "write", "list", "find", "test"],
                "operations": {
                    "read": ["content", "metadata", "diff"],
                    "write": ["put", "mkdir", "copy", "move", "delete", "touch", "archive", "unarchive"],
                    "list": ["entries", "system_info"],
                    "find": ["search"],
                    "test": ["echo", "generate_error"]
                }
            },
            "config": {
                "allowed_paths": config.allowed_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "max_file_read_bytes": config.max_file_read_bytes,
                "max_url_download_bytes": config.max_url_download_bytes,
                "http_timeout_ms": config.http_timeout_ms,
                "max_recursive_depth": config.max_recursive_depth,
                "max_find_results": config.max_find_results,
                "default_checksum": config.default_checksum,
            }
        }),
        "filesystem_stats" => {
            let allowed: Vec<Value> = config.allowed_paths.iter().map(|p| {
                json!({ "path": p.display().to_string(), "exists": p.exists() })
            }).collect();
            json!({ "allowed_paths": allowed })
        }
        other => return Ok(ToolResult::err(format!("Unknown info_type: {other}"))),
    };

    Ok(ToolResult::ok(serde_json::to_string_pretty(&result).unwrap_or_default()))
}
