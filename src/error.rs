use thiserror::Error;

/// All errors that can occur in coven-scout.
#[derive(Debug, Error)]
pub enum CovenScoutError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("File too large: {size} bytes (max {max})")]
    FileTooLarge { size: u64, max: u64 },

    #[error("Archive error: {0}")]
    ArchiveError(String),

    #[error("Checksum error: {0}")]
    ChecksumError(String),

    #[error("Image error: {0}")]
    ImageError(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl CovenScoutError {
    /// Return a user-facing string representation for MCP tool errors.
    pub fn to_tool_error(&self) -> String {
        self.to_string()
    }
}
