use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder, EntryType};
use crate::checksum;
use crate::error::LibrawdistError;
use crate::fs::FileSystem;
use crate::types::LibrawdistConfig;

/// Create a `.rawdist` package from a source directory.
/// The source directory must contain a valid `Librawdist.conf`.
/// The resulting archive is written to `output_path`.
pub fn create_package(
    fs: &dyn FileSystem,
    src_dir: &Path,
    output_path: &Path,
    config: &LibrawdistConfig,
) -> Result<(), LibrawdistError> {
    log::info!("Packing directory: {}", src_dir.display());

    // Generate checksums for files that will be included
    let checksums = checksum::generate_checksums(fs, src_dir, &config.files)?;
    let checksum_content = checksum::format_checksums(&checksums);
    log::debug!("Including {} files", checksums.len());

    let file = std::fs::File::create(output_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    for (rel, _) in &checksums {
        let full = src_dir.join(rel);
        tar.append_path_with_name(&full, rel)?;
    }

    // Add checksums.sha256 as an archive entry
    let mut header = tar::Header::new_gnu();
    header.set_path("checksums.sha256")?;
    header.set_size(checksum_content.len() as u64);
    header.set_cksum();
    tar.append_data(&mut header, "checksums.sha256", checksum_content.as_bytes())?;

    let enc = tar.into_inner()?;
    enc.finish()?;
    log::info!("Package created: {}", output_path.display());
    Ok(())
}

/// Extract a `.rawdist` archive to a temporary directory, verify checksums,
/// and return the persistent path to the extracted contents.
/// The caller is responsible for cleaning up the returned directory.
pub fn extract_to_temp(archive_path: &Path) -> Result<PathBuf, LibrawdistError> {
    let temp_dir = tempfile::tempdir()?;
    let dest = temp_dir.path().to_path_buf();

    let file = std::fs::File::open(archive_path)?;
    let dec = GzDecoder::new(file);
    let mut archive = Archive::new(dec);
    // Safe extraction: prevent path traversal
    let dest_canonical = dest.canonicalize()?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        // Resolve the intended destination
        let joined = dest.join(&path);
        // Canonicalize the parent directory first, then check
        let canonical_dest = match joined.parent() {
            Some(parent) => {
                // If parent doesn't exist, we'll create it later, but we must ensure safety.
                // First, check that the joined path does not contain '..'
                if path.components().any(|c| c == std::path::Component::ParentDir) {
                    return Err(LibrawdistError::PathTraversal(path));
                }
                // We'll create directories and then check
                std::fs::create_dir_all(parent)?;
                parent.canonicalize()
            }
            None => {
                // File in root of dest
                dest.canonicalize()
            }
        }?;
        // Verify the canonicalized parent is within dest_canonical
        if !canonical_dest.starts_with(&dest_canonical) {
            return Err(LibrawdistError::PathTraversal(path));
        }
        // Now extract the entry
        entry.unpack_in(&dest)?;
    }

    // Verify checksums
    let checksum_file = dest.join("checksums.sha256");
    if !checksum_file.exists() {
        return Err(LibrawdistError::MissingFile {
            path: PathBuf::from("checksums.sha256"),
        });
    }
    let content = std::fs::read_to_string(&checksum_file)?;
    let expected = checksum::parse_checksums(&content)?;
    for (rel_path, expected_hash) in &expected {
        let actual_path = dest.join(rel_path);
        if !actual_path.exists() {
            return Err(LibrawdistError::MissingFile {
                path: rel_path.clone(),
            });
        }
        let actual_hash = checksum::hash_file(&crate::fs::RealFs, &actual_path)?; // use RealFs for now
        if &actual_hash != expected_hash {
            return Err(LibrawdistError::ChecksumMismatch {
                path: actual_path,
            });
        }
    }
    // Remove checksum file (we don't need it after verification)
    std::fs::remove_file(&checksum_file)?;
    // Convert TempDir to a permanent path (prevent automatic deletion)
    let persistent = temp_dir.into_path();
    Ok(persistent)
}

/// Move the extracted package directory to its final install location.
/// This is an atomic rename if the source and destination are on the same filesystem.
pub fn move_extracted(src: &Path, target: &Path) -> Result<(), LibrawdistError> {
    if target.exists() {
        return Err(LibrawdistError::Config(format!(
            "Target already exists: {}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(src, target)?;
    Ok(())
}
