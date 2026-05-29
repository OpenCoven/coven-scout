use std::path::{Path, PathBuf};
use crate::{Config, CovenScoutError};

/// Expand `~` in a path string to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(path).as_ref())
}

/// Validate and canonicalize a path against the allowed prefixes.
///
/// Returns the absolute, resolved `PathBuf` if allowed.
/// Returns `CovenScoutError::SecurityViolation` if not within any allowed prefix.
pub fn validate_path(path: &str, config: &Config) -> Result<PathBuf, CovenScoutError> {
    let expanded = expand_tilde(path);

    // Make absolute relative to cwd if needed
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map_err(|e| CovenScoutError::Io(e))?
            .join(expanded)
    };

    // Resolve as much as possible — canonicalize requires existence, so we use a best-effort approach
    let resolved = best_effort_canonicalize(&absolute);

    // Check against allowed paths
    for allowed in &config.allowed_paths {
        let allowed_canon = best_effort_canonicalize(allowed);
        if resolved.starts_with(&allowed_canon) {
            return Ok(resolved);
        }
    }

    Err(CovenScoutError::SecurityViolation(format!(
        "Path '{}' is not within any allowed directory. Allowed: {}",
        resolved.display(),
        config
            .allowed_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Canonicalize path without requiring it to exist.
/// Resolves `..` and `.` components lexically, then tries `std::fs::canonicalize` for the
/// existing prefix.
pub fn best_effort_canonicalize(path: &Path) -> PathBuf {
    // First try full canonicalize
    if let Ok(canon) = std::fs::canonicalize(path) {
        return canon;
    }

    // Walk up to find existing prefix, canonicalize that, then reattach suffix
    let mut components: Vec<_> = path.components().collect();
    let mut suffix: Vec<std::path::Component> = Vec::new();

    loop {
        let candidate: PathBuf = components.iter().collect();
        if candidate.exists() {
            if let Ok(canon) = std::fs::canonicalize(&candidate) {
                let mut result = canon;
                for c in suffix.into_iter().rev() {
                    result = result.join(c);
                }
                return result;
            }
        }
        if components.is_empty() {
            break;
        }
        if let Some(last) = components.pop() {
            suffix.push(last);
        } else {
            break;
        }
    }

    // Fallback: lexical normalization
    normalize_lexically(path)
}

/// Lexically normalize a path (resolve `.` and `..` without touching the filesystem).
pub fn normalize_lexically(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            c => result.push(c),
        }
    }
    result
}
