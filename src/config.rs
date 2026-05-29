use std::path::PathBuf;

/// Runtime configuration for coven-scout.
#[derive(Debug, Clone)]
pub struct Config {
    /// Allowed path prefixes for filesystem operations.
    pub allowed_paths: Vec<PathBuf>,
    /// Log level string (TRACE/DEBUG/INFO/WARN/ERROR).
    pub log_level: String,
    /// Log file path, or None for stderr.
    pub log_file: Option<PathBuf>,
    /// Maximum bytes to read from a single file.
    pub max_file_read_bytes: u64,
    /// Maximum bytes to download from a URL.
    pub max_url_download_bytes: u64,
    /// HTTP timeout in milliseconds.
    pub http_timeout_ms: u64,
    /// Maximum recursive depth for listings/finds.
    pub max_recursive_depth: u32,
    /// Maximum results from find.
    pub max_find_results: usize,
    /// Default checksum algorithm.
    pub default_checksum: String,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        Self {
            allowed_paths: vec![home, PathBuf::from("/tmp")],
            log_level: "INFO".to_string(),
            log_file: None,
            max_file_read_bytes: 52_428_800,  // 50MB
            max_url_download_bytes: 20_971_520, // 20MB
            http_timeout_ms: 30_000,
            max_recursive_depth: 10,
            max_find_results: 1000,
            default_checksum: "sha256".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(paths) = std::env::var("COVEN_SCOUT_ALLOWED_PATHS") {
            let expanded: Vec<PathBuf> = paths
                .split(':')
                .filter(|s| !s.is_empty())
                .map(|s| {
                    let expanded = shellexpand::tilde(s);
                    PathBuf::from(expanded.as_ref())
                })
                .collect();
            if !expanded.is_empty() {
                cfg.allowed_paths = expanded;
            }
        }

        if let Ok(level) = std::env::var("COVEN_SCOUT_LOG_LEVEL") {
            cfg.log_level = level;
        }

        if let Ok(file) = std::env::var("COVEN_SCOUT_LOG_FILE") {
            if file.eq_ignore_ascii_case("NONE") {
                cfg.log_file = None; // means "no file, already None"
            } else {
                cfg.log_file = Some(PathBuf::from(file));
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_MAX_FILE_READ_BYTES") {
            if let Ok(n) = val.parse() {
                cfg.max_file_read_bytes = n;
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_MAX_URL_DOWNLOAD_BYTES") {
            if let Ok(n) = val.parse() {
                cfg.max_url_download_bytes = n;
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_HTTP_TIMEOUT_MS") {
            if let Ok(n) = val.parse() {
                cfg.http_timeout_ms = n;
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_MAX_RECURSIVE_DEPTH") {
            if let Ok(n) = val.parse() {
                cfg.max_recursive_depth = n;
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_MAX_FIND_RESULTS") {
            if let Ok(n) = val.parse() {
                cfg.max_find_results = n;
            }
        }

        if let Ok(val) = std::env::var("COVEN_SCOUT_DEFAULT_CHECKSUM") {
            cfg.default_checksum = val;
        }

        cfg
    }

    /// Returns true if the default (home + /tmp) paths are in use.
    pub fn is_using_defaults(&self) -> bool {
        std::env::var("COVEN_SCOUT_ALLOWED_PATHS").is_err()
    }
}
