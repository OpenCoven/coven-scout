use std::path::Path;
use sha2::{Digest as Sha2Digest, Sha256, Sha512};
use md5::Md5;
use sha2::Sha384;
use crate::CovenScoutError;

/// Compute a hex checksum of the given bytes using the named algorithm.
pub fn checksum_bytes(data: &[u8], algorithm: &str) -> Result<String, CovenScoutError> {
    match algorithm.to_lowercase().as_str() {
        "sha256" => {
            let mut h = Sha256::new();
            h.update(data);
            Ok(hex::encode(h.finalize()))
        }
        "sha512" => {
            let mut h = Sha512::new();
            h.update(data);
            Ok(hex::encode(h.finalize()))
        }
        "md5" => {
            let mut h = Md5::new();
            h.update(data);
            Ok(hex::encode(h.finalize()))
        }
        other => Err(CovenScoutError::ChecksumError(format!(
            "Unknown algorithm: {other}. Supported: sha256, sha512, md5"
        ))),
    }
}

/// Compute a hex checksum of a file at the given path.
pub async fn checksum_file(path: &Path, algorithm: &str) -> Result<String, CovenScoutError> {
    let data = tokio::fs::read(path).await?;
    checksum_bytes(&data, algorithm)
}
