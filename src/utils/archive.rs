use std::io::{Read, Write};
use std::path::Path;
use crate::CovenScoutError;

/// Create a zip archive from source paths into `archive_path`.
pub fn create_zip(source_paths: &[impl AsRef<Path>], archive_path: &Path) -> Result<(), CovenScoutError> {
    let file = std::fs::File::create(archive_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for src in source_paths {
        let src = src.as_ref();
        if src.is_dir() {
            add_dir_to_zip(&mut zip, src, src, &options)?;
        } else if src.is_file() {
            add_file_to_zip(&mut zip, src, src.file_name().unwrap_or_default().to_string_lossy().as_ref(), &options)?;
        }
    }

    zip.finish().map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
    Ok(())
}

fn add_file_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    file_path: &Path,
    name_in_zip: &str,
    options: &zip::write::SimpleFileOptions,
) -> Result<(), CovenScoutError> {
    zip.start_file(name_in_zip, *options)
        .map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
    let mut f = std::fs::File::open(file_path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: &zip::write::SimpleFileOptions,
) -> Result<(), CovenScoutError> {
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| CovenScoutError::Io(e.into()))?;
        let path = entry.path();
        let relative = path.strip_prefix(base)
            .map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
        let name = relative.to_string_lossy();
        if name.is_empty() {
            continue;
        }
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), *options)
                .map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
        } else {
            add_file_to_zip(zip, path, &name, options)?;
        }
    }
    Ok(())
}

/// Extract a zip archive into `destination_path`.
pub fn extract_zip(archive_path: &Path, destination_path: &Path) -> Result<(), CovenScoutError> {
    let file = std::fs::File::open(archive_path)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
    zip.extract(destination_path)
        .map_err(|e| CovenScoutError::ArchiveError(e.to_string()))?;
    Ok(())
}

/// Create a tar.gz archive from source paths.
pub fn create_tar_gz(source_paths: &[impl AsRef<Path>], archive_path: &Path) -> Result<(), CovenScoutError> {
    let file = std::fs::File::create(archive_path)?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);

    for src in source_paths {
        let src = src.as_ref();
        if src.is_dir() {
            let dir_name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
            tar.append_dir_all(&dir_name, src)?;
        } else if src.is_file() {
            let mut f = std::fs::File::open(src)?;
            let name = src.file_name().unwrap_or_default().to_string_lossy().to_string();
            tar.append_file(&name, &mut f)?;
        }
    }

    tar.finish()?;
    Ok(())
}

/// Extract a tar.gz archive into `destination_path`.
pub fn extract_tar_gz(archive_path: &Path, destination_path: &Path) -> Result<(), CovenScoutError> {
    let file = std::fs::File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    tar.unpack(destination_path)?;
    Ok(())
}
