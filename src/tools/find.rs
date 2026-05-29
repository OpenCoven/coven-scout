use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{Config, CovenScoutError};
use crate::protocol::{Tool, ToolResult};
use crate::security::validate_path;
use tracing::instrument;
use regex::Regex;

pub fn tool_descriptor() -> Tool {
    Tool {
        name: "find".to_string(),
        description: "Search for files matching various criteria (name, content, size, date, MIME type).".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": { "type": "string", "enum": ["search"] },
                "path": { "type": "string", "description": "Root path to search" },
                "recursive": { "type": "boolean" },
                "name_pattern": { "type": "string", "description": "Glob pattern for name" },
                "case_sensitive": { "type": "boolean" },
                "content_pattern": { "type": "string" },
                "content_is_regex": { "type": "boolean" },
                "content_case_sensitive": { "type": "boolean" },
                "file_extensions": { "type": "array", "items": { "type": "string" } },
                "size_min": { "type": "integer" },
                "size_max": { "type": "integer" },
                "modified_after": { "type": "string" },
                "modified_before": { "type": "string" },
                "entry_type": { "type": "string", "enum": ["file", "directory", "any"] },
                "mime_type": { "type": "string" },
                "max_results": { "type": "integer" }
            },
            "required": ["operation", "path"]
        }),
    }
}

#[derive(Debug, Deserialize)]
struct FindArgs {
    operation: String,
    path: String,
    recursive: Option<bool>,
    name_pattern: Option<String>,
    case_sensitive: Option<bool>,
    content_pattern: Option<String>,
    content_is_regex: Option<bool>,
    content_case_sensitive: Option<bool>,
    file_extensions: Option<Vec<String>>,
    size_min: Option<u64>,
    size_max: Option<u64>,
    modified_after: Option<String>,
    modified_before: Option<String>,
    entry_type: Option<String>,
    mime_type: Option<String>,
    max_results: Option<usize>,
}

#[instrument(skip(config))]
pub async fn handle(args: Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let args: FindArgs = serde_json::from_value(args)
        .map_err(|e| CovenScoutError::InvalidArgument(e.to_string()))?;

    match args.operation.as_str() {
        "search" => handle_search(args, config).await,
        other => Ok(ToolResult::err(format!("Unknown find operation: {other}"))),
    }
}

async fn handle_search(args: FindArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let root = validate_path(&args.path, &config)?;
    let recursive = args.recursive.unwrap_or(true);
    let case_sensitive = args.case_sensitive.unwrap_or(false);
    let max_results = args.max_results.unwrap_or(config.max_find_results).min(config.max_find_results);

    // Parse date filters
    let modified_after = args.modified_after.as_deref()
        .map(|s| chrono::DateTime::parse_from_rfc3339(s).map(|d| d.with_timezone(&chrono::Utc)))
        .transpose()
        .map_err(|e| CovenScoutError::InvalidArgument(format!("modified_after parse error: {e}")))?;

    let modified_before = args.modified_before.as_deref()
        .map(|s| chrono::DateTime::parse_from_rfc3339(s).map(|d| d.with_timezone(&chrono::Utc)))
        .transpose()
        .map_err(|e| CovenScoutError::InvalidArgument(format!("modified_before parse error: {e}")))?;

    // Compile content pattern
    let content_regex: Option<Regex> = if let Some(pattern) = &args.content_pattern {
        let is_regex = args.content_is_regex.unwrap_or(false);
        let content_case = args.content_case_sensitive.unwrap_or(false);
        let pattern_str = if is_regex {
            if content_case {
                pattern.clone()
            } else {
                format!("(?i){pattern}")
            }
        } else {
            let escaped = regex::escape(pattern);
            if content_case { escaped } else { format!("(?i){escaped}") }
        };
        Some(Regex::new(&pattern_str)?)
    } else {
        None
    };

    let entry_type = args.entry_type.as_deref().unwrap_or("any");
    let max_depth = if recursive { config.max_recursive_depth as usize } else { 1 };

    let mut walker = walkdir::WalkDir::new(&root).max_depth(max_depth);
    let mut results = Vec::new();

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Entry type filter
        let is_dir = meta.is_dir();
        let is_file = meta.is_file();
        match entry_type {
            "file" => if !is_file { continue; }
            "directory" => if !is_dir { continue; }
            _ => {}
        }

        // Name pattern filter
        if let Some(pattern) = &args.name_pattern {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let (name_cmp, pattern_cmp) = if case_sensitive {
                (name.to_string(), pattern.clone())
            } else {
                (name.to_lowercase(), pattern.to_lowercase())
            };
            let glob_match = glob::Pattern::new(&pattern_cmp)
                .map(|p| p.matches(&name_cmp))
                .unwrap_or(false);
            if !glob_match {
                continue;
            }
        }

        // Extension filter
        if let Some(exts) = &args.file_extensions {
            let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
            let exts_lower: Vec<String> = exts.iter().map(|e| e.trim_start_matches('.').to_lowercase()).collect();
            if !exts_lower.contains(&ext.to_string()) {
                continue;
            }
        }

        // Size filters
        if is_file {
            let size = meta.len();
            if let Some(min) = args.size_min {
                if size < min { continue; }
            }
            if let Some(max) = args.size_max {
                if size > max { continue; }
            }
        }

        // Modified time filters
        if let Ok(modified) = meta.modified() {
            let dt: chrono::DateTime<chrono::Utc> = modified.into();
            if let Some(after) = modified_after {
                if dt < after { continue; }
            }
            if let Some(before) = modified_before {
                if dt > before { continue; }
            }
        }

        // MIME type filter
        if let Some(mime_filter) = &args.mime_type {
            let mime = mime_guess::from_path(path).first().map(|m| m.to_string()).unwrap_or_default();
            if !mime.contains(mime_filter.as_str()) {
                continue;
            }
        }

        // Content pattern filter (files only)
        if let Some(re) = &content_regex {
            if !is_file { continue; }
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if !re.is_match(&content) { continue; }
                }
                Err(_) => continue, // Skip binary/unreadable files
            }
        }

        let mime = if is_file {
            mime_guess::from_path(path).first().map(|m| m.to_string()).unwrap_or_else(|| "application/octet-stream".to_string())
        } else {
            "inode/directory".to_string()
        };

        let modified_str = meta.modified().ok().map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });

        results.push(json!({
            "path": path.display().to_string(),
            "name": path.file_name().unwrap_or_default().to_string_lossy(),
            "type": if is_dir { "directory" } else { "file" },
            "size": meta.len(),
            "mime_type": mime,
            "modified": modified_str,
        }));
    }

    let output = json!({
        "count": results.len(),
        "results": results,
    });

    Ok(ToolResult::ok(serde_json::to_string_pretty(&output).unwrap_or_default()))
}
