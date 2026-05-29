use std::time::Duration;
use crate::{Config, CovenScoutError};

/// Fetch a URL and return the body bytes.
pub async fn fetch_url(url: &str, config: &Config) -> Result<bytes::Bytes, CovenScoutError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.http_timeout_ms))
        .user_agent("coven-scout/0.1.0")
        .build()?;

    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        return Err(CovenScoutError::InvalidArgument(format!(
            "HTTP {status} for URL: {url}"
        )));
    }

    // Check content-length header first
    if let Some(len) = response.content_length() {
        if len > config.max_url_download_bytes {
            return Err(CovenScoutError::FileTooLarge {
                size: len,
                max: config.max_url_download_bytes,
            });
        }
    }

    let bytes = response.bytes().await?;

    if bytes.len() as u64 > config.max_url_download_bytes {
        return Err(CovenScoutError::FileTooLarge {
            size: bytes.len() as u64,
            max: config.max_url_download_bytes,
        });
    }

    Ok(bytes)
}

/// Fetch a URL and convert HTML to Markdown if the content-type is HTML.
pub async fn fetch_as_markdown(url: &str, config: &Config) -> Result<String, CovenScoutError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.http_timeout_ms))
        .user_agent("coven-scout/0.1.0")
        .build()?;

    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CovenScoutError::InvalidArgument(format!(
            "HTTP {status} for URL: {url}"
        )));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if let Some(len) = response.content_length() {
        if len > config.max_url_download_bytes {
            return Err(CovenScoutError::FileTooLarge {
                size: len,
                max: config.max_url_download_bytes,
            });
        }
    }

    let body = response.text().await?;

    if content_type.contains("html") {
        htmd::convert(&body).map_err(|e| CovenScoutError::Other(anyhow::anyhow!("htmd: {e}")))
    } else {
        Ok(body)
    }
}

/// Fetch URL as plain text.
pub async fn fetch_as_text(url: &str, config: &Config) -> Result<String, CovenScoutError> {
    let bytes = fetch_url(url, config).await?;
    String::from_utf8(bytes.to_vec())
        .map_err(|e| CovenScoutError::InvalidArgument(format!("Non-UTF8 content: {e}")))
}
