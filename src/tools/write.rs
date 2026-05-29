use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{Config, CovenScoutError};
use crate::protocol::{Tool, ToolResult};
use crate::security::validate_path;
use crate::utils::archive;
use tracing::instrument;

pub fn tool_descriptor() -> Tool {
    Tool {
        name: "write".to_string(),
        description: "Write files, create directories, copy/move/delete, archive/unarchive.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["put", "mkdir", "copy", "move", "delete", "touch", "archive", "unarchive"],
                    "description": "Operation to perform"
                },
                "entries": {
                    "type": "array",
                    "description": "List of entries for batch operations",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "content": { "type": "string" },
                            "write_mode": { "type": "string", "enum": ["overwrite", "append", "error_if_exists"] },
                            "input_encoding": { "type": "string", "enum": ["text", "base64"] },
                            "recursive": { "type": "boolean" },
                            "source_path": { "type": "string" },
                            "destination_path": { "type": "string" }
                        }
                    }
                },
                "source_paths": { "type": "array", "items": { "type": "string" } },
                "archive_path": { "type": "string" },
                "destination_path": { "type": "string" },
                "format": { "type": "string", "enum": ["zip", "tar.gz", "tgz"] }
            },
            "required": ["operation"]
        }),
    }
}

#[derive(Debug, Deserialize, Default)]
struct WriteEntry {
    path: Option<String>,
    content: Option<String>,
    write_mode: Option<String>,
    input_encoding: Option<String>,
    recursive: Option<bool>,
    source_path: Option<String>,
    destination_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WriteArgs {
    operation: String,
    #[serde(default)]
    entries: Vec<WriteEntry>,
    source_paths: Option<Vec<String>>,
    archive_path: Option<String>,
    destination_path: Option<String>,
    format: Option<String>,
}

#[instrument(skip(config))]
pub async fn handle(args: Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let args: WriteArgs = serde_json::from_value(args)
        .map_err(|e| CovenScoutError::InvalidArgument(e.to_string()))?;

    match args.operation.as_str() {
        "put" => handle_put(args.entries, &config).await,
        "mkdir" => handle_mkdir(args.entries, &config).await,
        "copy" => handle_copy(args.entries, &config).await,
        "move" => handle_move(args.entries, &config).await,
        "delete" => handle_delete(args.entries, &config).await,
        "touch" => handle_touch(args.entries, &config).await,
        "archive" => handle_archive(args, &config).await,
        "unarchive" => handle_unarchive(args, &config).await,
        other => Ok(ToolResult::err(format!("Unknown write operation: {other}"))),
    }
}

async fn handle_put(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let path_str = entry.path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("put entry missing 'path'".to_string()))?;
        let resolved = validate_path(path_str, config)?;
        let content = entry.content.unwrap_or_default();
        let encoding = entry.input_encoding.as_deref().unwrap_or("text");
        let mode = entry.write_mode.as_deref().unwrap_or("overwrite");

        // Ensure parent exists
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if mode == "error_if_exists" && resolved.exists() {
            results.push(format!("ERROR: {path_str}: file already exists"));
            continue;
        }

        let bytes: Vec<u8> = if encoding == "base64" {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(&content)
                .map_err(|e| CovenScoutError::InvalidArgument(format!("base64 decode: {e}")))?
        } else {
            content.into_bytes()
        };

        if mode == "append" {
            use tokio::io::AsyncWriteExt;
            let mut f = tokio::fs::OpenOptions::new()
                .create(true).append(true).open(&resolved).await?;
            f.write_all(&bytes).await?;
        } else {
            tokio::fs::write(&resolved, &bytes).await?;
        }

        results.push(format!("OK: {path_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

async fn handle_mkdir(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let path_str = entry.path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("mkdir entry missing 'path'".to_string()))?;
        let resolved = validate_path(path_str, config)?;
        let recursive = entry.recursive.unwrap_or(true);
        if recursive {
            tokio::fs::create_dir_all(&resolved).await?;
        } else {
            tokio::fs::create_dir(&resolved).await?;
        }
        results.push(format!("OK: {path_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

async fn handle_copy(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let src_str = entry.source_path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("copy entry missing 'source_path'".to_string()))?;
        let dst_str = entry.destination_path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("copy entry missing 'destination_path'".to_string()))?;
        let src = validate_path(src_str, config)?;
        let dst = validate_path(dst_str, config)?;

        if src.is_dir() {
            copy_dir_all(&src, &dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(&src, &dst).await?;
        }
        results.push(format!("OK: {src_str} -> {dst_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<(), CovenScoutError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

async fn handle_move(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let src_str = entry.source_path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("move entry missing 'source_path'".to_string()))?;
        let dst_str = entry.destination_path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("move entry missing 'destination_path'".to_string()))?;
        let src = validate_path(src_str, config)?;
        let dst = validate_path(dst_str, config)?;
        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&src, &dst).await?;
        results.push(format!("OK: {src_str} -> {dst_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

async fn handle_delete(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let path_str = entry.path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("delete entry missing 'path'".to_string()))?;
        let resolved = validate_path(path_str, config)?;
        let recursive = entry.recursive.unwrap_or(false);

        if resolved.is_dir() {
            if recursive {
                tokio::fs::remove_dir_all(&resolved).await?;
            } else {
                tokio::fs::remove_dir(&resolved).await?;
            }
        } else {
            tokio::fs::remove_file(&resolved).await?;
        }
        results.push(format!("OK: {path_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

async fn handle_touch(entries: Vec<WriteEntry>, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();
    for entry in entries {
        let path_str = entry.path.as_deref().ok_or_else(|| CovenScoutError::InvalidArgument("touch entry missing 'path'".to_string()))?;
        let resolved = validate_path(path_str, config)?;
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Create or update mtime
        if resolved.exists() {
            let now = std::time::SystemTime::now();
            // Update accessed + modified times via file open
            let _f = tokio::fs::OpenOptions::new().write(true).open(&resolved).await?;
            // Best effort; on most platforms touching the file is enough
        } else {
            tokio::fs::File::create(&resolved).await?;
        }
        results.push(format!("OK: {path_str}"));
    }
    Ok(ToolResult::ok(results.join("\n")))
}

async fn handle_archive(args: WriteArgs, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let source_paths = args.source_paths.ok_or_else(|| CovenScoutError::InvalidArgument("archive requires 'source_paths'".to_string()))?;
    let archive_path_str = args.archive_path.ok_or_else(|| CovenScoutError::InvalidArgument("archive requires 'archive_path'".to_string()))?;
    let format = args.format.as_deref().unwrap_or("zip");

    let archive_path = validate_path(&archive_path_str, config)?;
    let resolved_sources: Vec<std::path::PathBuf> = source_paths.iter()
        .map(|p| validate_path(p, config))
        .collect::<Result<_, _>>()?;

    match format {
        "zip" => tokio::task::spawn_blocking(move || archive::create_zip(&resolved_sources, &archive_path)).await
            .map_err(|e| CovenScoutError::Other(anyhow::anyhow!(e)))??
        ,
        "tar.gz" | "tgz" => tokio::task::spawn_blocking(move || archive::create_tar_gz(&resolved_sources, &archive_path)).await
            .map_err(|e| CovenScoutError::Other(anyhow::anyhow!(e)))??
        ,
        other => return Ok(ToolResult::err(format!("Unknown archive format: {other}"))),
    }

    Ok(ToolResult::ok(format!("Archive created: {archive_path_str}")))
}

async fn handle_unarchive(args: WriteArgs, config: &Config) -> Result<ToolResult, CovenScoutError> {
    let archive_path_str = args.archive_path.ok_or_else(|| CovenScoutError::InvalidArgument("unarchive requires 'archive_path'".to_string()))?;
    let dst_str = args.destination_path.ok_or_else(|| CovenScoutError::InvalidArgument("unarchive requires 'destination_path'".to_string()))?;

    let archive_path = validate_path(&archive_path_str, config)?;
    let dst_path = validate_path(&dst_str, config)?;

    // Detect format from args or extension
    let format = args.format.clone().unwrap_or_else(|| {
        let name = archive_path.to_string_lossy().to_lowercase();
        if name.ends_with(".zip") { "zip".to_string() }
        else if name.ends_with(".tar.gz") || name.ends_with(".tgz") { "tar.gz".to_string() }
        else { "zip".to_string() }
    });

    tokio::fs::create_dir_all(&dst_path).await?;

    match format.as_str() {
        "zip" => tokio::task::spawn_blocking(move || archive::extract_zip(&archive_path, &dst_path)).await
            .map_err(|e| CovenScoutError::Other(anyhow::anyhow!(e)))??
        ,
        "tar.gz" | "tgz" => tokio::task::spawn_blocking(move || archive::extract_tar_gz(&archive_path, &dst_path)).await
            .map_err(|e| CovenScoutError::Other(anyhow::anyhow!(e)))??
        ,
        other => return Ok(ToolResult::err(format!("Unknown archive format: {other}"))),
    }

    Ok(ToolResult::ok(format!("Extracted to: {dst_str}")))
}
