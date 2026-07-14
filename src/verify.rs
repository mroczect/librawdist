use crate::error::RawdistError;
use crate::fs::FileSystem;
use std::path::Path;

/// Verifies the integrity of a `.rawdist` archive without installing it.
///
/// This function extracts the archive to a temporary directory, performs
/// all standard integrity checks (archive size, path traversal prevention,
/// checksum verification against the embedded `checksums.sha256`), and then
/// either retains or deletes the extracted content based on the `keep_temp`
/// flag.
///
/// It is designed for diagnostic and pre‑installation inspection workflows.
/// No manifest or lock file changes occur.
///
/// # Arguments
///
/// * `fs` – A [`FileSystem`] implementation used for I/O operations.
/// * `archive_path` – Path to the `.rawdist` archive to verify.
/// * `keep_temp` – If `true`, the temporary directory containing the extracted
///   and verified files is preserved and its path returned as `Some(path)`.
///   If `false`, the directory is deleted after successful verification and
///   `None` is returned.
///
/// # Returns
///
/// * `Ok(Some(PathBuf))` – Verification succeeded, and the extracted files are
///   kept at the returned path. The caller is responsible for removing the
///   directory eventually.
/// * `Ok(None)` – Verification succeeded and the temporary directory was
///   cleaned up.
/// * `Err(RawdistError)` – If the archive is missing, too large, contains path
///   traversal attempts, fails checksum validation, or encounters I/O errors.
///
/// # Panics
///
/// This function does not panic. All error conditions are returned via
/// `Result::Err`.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::verify::verify_package;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let result = verify_package(&fs, Path::new("pkg.rawdist"), false)
///     .expect("Verification failed");
/// assert!(result.is_none());
/// ```
pub fn verify_package(
    fs: &dyn FileSystem,
    archive_path: &Path,
    keep_temp: bool,
) -> Result<Option<std::path::PathBuf>, RawdistError> {
    // Explicitly check for archive existence to provide a clear, early
    // diagnostic instead of a confusing “file not found” error during
    // extraction.
    if !fs.exists(archive_path) {
        return Err(RawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }

    // Delegate extraction and verification to the package module.
    // `extract_to_temp` returns a persistent temporary directory path
    // after all checks pass.
    let extracted = crate::package::extract_to_temp(fs, archive_path)?;

    if keep_temp {
        // The user requested to keep the extracted content for inspection.
        // Return the path so they can examine the files; cleanup is their
        // responsibility.
        log::info!(
            "Verification successful. Extracted at {}",
            extracted.display()
        );
        Ok(Some(extracted))
    } else {
        // Default behaviour: remove the temporary directory immediately.
        // A failure here is reported as an I/O error, but the archive
        // itself was already fully verified.
        fs.remove_dir_all(&extracted)?;
        log::info!("Verification successful: all checksums match.");
        Ok(None)
    }
}
