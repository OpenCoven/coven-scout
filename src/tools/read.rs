use std::sync::Arc;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{Config, CovenScoutError};
use crate::protocol::{Tool, ToolResult};
use crate::security::validate_path;
use crate::utils::{checksum, diff, web};
use tracing::instrument;

pub fn tool_descriptor() -> Tool {
    Tool {
        name: "read".to_string(),
        description: "Read files or URLs, get metadata, or compute diffs.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["content", "metadata", "diff"],
                    "description": "Operation to perform"
                },
                "sources": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "File paths or URLs to read"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "base64", "markdown", "checksum"],
                    "description": "Output format for content operation"
                },
                "checksum_algorithm": {
                    "type": "string",
                    "enum": ["sha256", "sha512", "md5"],
                    "description": "Algorithm for checksum format"
                },
                "offset": { "type": "integer", "description": "Byte offset for partial reads" },
                "length": { "type": "integer", "description": "Max bytes to read" }
            },
            "required": ["operation", "sources"]
        }),
    }
}

#[derive(Debug, Deserialize)]
struct ReadArgs {
    operation: String,
    sources: Vec<String>,
    format: Option<String>,
    checksum_algorithm: Option<String>,
    offset: Option<u64>,
    length: Option<u64>,
}

#[instrument(skip(config))]
pub async fn handle(args: Value, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let args: ReadArgs = serde_json::from_value(args)
        .map_err(|e| CovenScoutError::InvalidArgument(e.to_string()))?;

    match args.operation.as_str() {
        "content" => handle_content(args, config).await,
        "metadata" => handle_metadata(args, config).await,
        "diff" => handle_diff(args, config).await,
        other => Ok(ToolResult::err(format!("Unknown read operation: {other}"))),
    }
}

async fn handle_content(args: ReadArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let format = args.format.as_deref().unwrap_or("text");
    let algo = args.checksum_algorithm.as_deref()
        .unwrap_or(&config.default_checksum);

    let mut results = Vec::new();

    for source in &args.sources {
        let result = if source.starts_with("http://") || source.starts_with("https://") {
            handle_url_content(source, format, &config).await
        } else {
            handle_file_content(source, format, algo, args.offset, args.length, &config).await
        };

        match result {
            Ok(text) => results.push(format!("=== {source} ===\n{text}")),
            Err(e) => results.push(format!("=== {source} ===\nERROR: {e}")),
        }
    }

    Ok(ToolResult::ok(results.join("\n\n")))
}

async fn handle_file_content(
    path: &str,
    format: &str,
    algo: &str,
    offset: Option<u64>,
    length: Option<u64>,
    config: &Config,
) -> Result<String, CovenScoutError> {
    let resolved = validate_path(path, config)?;

    if format == "checksum" {
        return checksum::checksum_file(&resolved, algo).await;
    }

    // Read with offset/length support
    let mut file = tokio::fs::File::open(&resolved).await?;
    let meta = tokio::fs::metadata(&resolved).await?;
    let file_size = meta.len();

    let start = offset.unwrap_or(0);
    let max_read = length.unwrap_or(config.max_file_read_bytes).min(config.max_file_read_bytes);

    if start > file_size {
        return Err(CovenScoutError::InvalidArgument(format!(
            "Offset {start} exceeds file size {file_size}"
        )));
    }

    let bytes_to_read = (file_size - start).min(max_read) as usize;

    use tokio::io::{AsyncReadExt, AsyncSeekExt};
    if start > 0 {
        file.seek(std::io::SeekFrom::Start(start)).await?;
    }

    let mut buf = vec![0u8; bytes_to_read];
    let n = file.read(&mut buf).await?;
    buf.truncate(n);

    match format {
        "base64" => Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf)),
        _ => {
            // text (default)
            String::from_utf8(buf)
                .map_err(|_| CovenScoutError::InvalidArgument("File is not valid UTF-8; use format=base64".to_string()))
        }
    }
}

async fn handle_url_content(url: &str, format: &str, config: &Config) -> Result<String, CovenScoutError> {
    match format {
        "markdown" => web::fetch_as_markdown(url, config).await,
        "base64" => {
            let bytes = web::fetch_url(url, config).await?;
            Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes))
        }
        "checksum" => {
            let bytes = web::fetch_url(url, config).await?;
            checksum::checksum_bytes(&bytes, &config.default_checksum)
        }
        _ => web::fetch_as_text(url, config).await,
    }
}

async fn handle_metadata(args: ReadArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    let mut results = Vec::new();

    for source in &args.sources {
        let info = if source.starts_with("http://") || source.starts_with("https://") {
            get_url_metadata(source, &config).await
        } else {
            get_file_metadata(source, &config).await
        };

        match info {
            Ok(v) => results.push(format!(
                "=== {source} ===\n{}",
                serde_json::to_string_pretty(&v).unwrap_or_default()
            )),
            Err(e) => results.push(format!("=== {source} ===\nERROR: {e}")),
        }
    }

    Ok(ToolResult::ok(results.join("\n\n")))
}

async fn get_file_metadata(path: &str, config: &Config) -> Result<Value, CovenScoutError> {
    let resolved = validate_path(path, config)?;
    let meta = tokio::fs::metadata(&resolved).await?;

    let mime = mime_guess::from_path(&resolved)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let modified = meta.modified().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Utc> = t.into();
        dt.to_rfc3339()
    });

    let created = meta.created().ok().map(|t| {
        let dt: chrono::DateTime<chrono::Utc> = t.into();
        dt.to_rfc3339()
    });

    let symlink_target = if meta.is_symlink() {
        tokio::fs::read_link(&resolved).await.ok().map(|p| p.display().to_string())
    } else {
        None
    };

    #[cfg(unix)]
    let (permissions, executable) = {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        (format!("{:o}", mode & 0o777), mode & 0o111 != 0)
    };

    #[cfg(not(unix))]
    let (permissions, executable) = ("N/A".to_string(), false);

    Ok(json!({
        "path": resolved.display().to_string(),
        "type": if meta.is_dir() { "directory" } else if meta.is_symlink() { "symlink" } else { "file" },
        "size": meta.len(),
        "mime_type": mime,
        "modified": modified,
        "created": created,
        "permissions": permissions,
        "executable": executable,
        "symlink_target": symlink_target,
        "readonly": meta.permissions().readonly(),
    }))
}

async fn get_url_metadata(url: &str, config: &Config) -> Result<Value, CovenScoutError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(config.http_timeout_ms))
        .user_agent("coven-scout/0.1.0")
        .build()?;

    let resp = client.head(url).send().await?;
    let status = resp.status().as_u16();
    let content_type = resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let content_length = resp.content_length();

    Ok(json!({
        "url": url,
        "http_status": status,
        "content_type": content_type,
        "content_length": content_length,
    }))
}

async fn handle_diff(args: ReadArgs, config: Arc<Config>) -> Result<ToolResult, CovenScoutError> {
    if args.sources.len() != 2 {
        return Ok(ToolResult::err("diff requires exactly 2 sources"));
    }

    let path_a = validate_path(&args.sources[0], &config)?;
    let path_b = validate_path(&args.sources[1], &config)?;

    let text_a = tokio::fs::read_to_string(&path_a).await?;
    let text_b = tokio::fs::read_to_string(&path_b).await?;

    let label_a = path_a.display().to_string();
    let label_b = path_b.display().to_string();

    let result = diff::unified_diff(&text_a, &text_b, &label_a, &label_b);
    Ok(ToolResult::ok(result))
}
