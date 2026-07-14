use crate::checksum;
use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::RawdistConfig;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder, EntryType};

/// Maximum allowed size (500 MiB) for an archive before extraction is refused.
///
/// This limit is a safety measure against decompression bombs—archives that
/// expand to enormous sizes from a relatively small compressed payload. The
/// value is checked against the *compressed* file size, which is a reasonable
/// first‑line defense for the typical `.rawdist` package size in the rawssg
/// ecosystem. If a legitimate package exceeds this size, the limit should be
/// raised after careful consideration.
// Reason: 500 MB is chosen as a generous upper bound for theme/plugin
// packages while still providing strong protection. The constant is kept
// module‑private to avoid exposing tuning knobs that could be misused.
const MAX_ARCHIVE_SIZE: u64 = 500 * 1024 * 1024;

/// Creates a `.rawdist` package from a source directory.
///
/// The function walks the source directory according to the include/exclude
/// patterns in `config.files`, generates a SHA‑256 checksum for every included
/// file, and packages everything together with a `checksums.sha256` manifest
/// into a gzip‑compressed tarball.
///
/// # Arguments
///
/// * `fs` – The [`FileSystem`] used to read source files and write the
///   archive.
/// * `src_dir` – The root directory containing the `rawdist.conf` and all
///   package files. Only files matching the include patterns (and not
///   excluded) are packed.
/// * `output_path` – The path where the resulting `.rawdist` archive will be
///   created. Parent directories are created automatically.
/// * `config` – The validated [`RawdistConfig`] that specifies file patterns
///   and package metadata. The config must already pass
///   [`RawdistConfig::validate`].
///
/// # Returns
///
/// * `Ok(())` – The archive was successfully created at `output_path`.
/// * `Err(RawdistError)` – On any I/O error, checksum generation failure,
///   or tar/gzip encoding failure.
///
/// # Panics
///
/// This function does not panic. All error conditions are returned as
/// `Result::Err`.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::package::create_package;
/// use librawdist::fs::RealFs;
/// use librawdist::types::RawdistConfig;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let config = RawdistConfig::load_from_dir(&fs, Path::new("./my_package"))
///     .expect("Failed to load config");
/// create_package(&fs, Path::new("./my_package"), Path::new("out.rawdist"), &config)
///     .expect("Package creation failed");
/// ```
pub fn create_package(
    fs: &dyn FileSystem,
    src_dir: &Path,
    output_path: &Path,
    config: &RawdistConfig,
) -> Result<(), RawdistError> {
    log::info!("Packing directory: {}", src_dir.display());

    // Generate checksums for all files that will be included. This step
    // also verifies that required files (like rawdist.conf) exist.
    let checksums = checksum::generate_checksums(fs, src_dir, &config.files)?;
    // Format the checksums as a standard "hash  path" text file for
    // inclusion in the archive.
    let checksum_content = checksum::format_checksums(&checksums);
    log::debug!("Including {} files", checksums.len());

    // Build the archive entirely in memory before writing to disk.
    // Using an in‑memory buffer simplifies error handling: if any part
    // of the tarball construction fails, nothing is written to
    // `output_path`, preventing partial or corrupt archives on disk.
    let mut archive_data = Vec::new();
    {
        // The GzEncoder is wrapped around the Vec to compress the
        // tar stream on‑the‑fly.
        let enc = GzEncoder::new(&mut archive_data, Compression::default());
        let mut tar = Builder::new(enc);

        // Add each file to the tar archive, preserving its relative
        // path so that extraction recreates the directory structure.
        for (rel, _) in &checksums {
            let full = src_dir.join(rel);
            tar.append_path_with_name(&full, rel)?;
        }

        // Append the checksums.sha256 file manually as a regular
        // entry. This ensures the file is present and can be used
        // later for integrity verification.
        let mut header = tar::Header::new_gnu();
        header.set_path("checksums.sha256")?;
        header.set_size(checksum_content.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, "checksums.sha256", checksum_content.as_bytes())?;

        // Finalize the tar and then the gzip encoder, ensuring all
        // data is flushed.
        let enc = tar.into_inner()?;
        enc.finish()?;
    }

    // Write the complete compressed archive to the output path.
    fs.write(output_path, &archive_data)?;
    log::info!("Package written to {}", output_path.display());
    Ok(())
}

/// Extracts a `.rawdist` archive to a persistent temporary directory and
/// verifies all file checksums.
///
/// This function performs several safety and integrity checks:
///
/// 1. Rejects archives larger than [`MAX_ARCHIVE_SIZE`] to prevent
///    decompression bombs.
/// 2. Rejects entries that contain `..` components (path traversal attempt).
/// 3. Rejects entries that are not regular files or directories (e.g.,
///    symlinks, hard links, devices).
/// 4. After extraction, every file listed in the embedded `checksums.sha256`
///    is verified against its expected hash.
///
/// If all checks pass, the temporary directory is kept and its path is
/// returned. The caller is responsible for eventually removing it (using
/// [`std::fs::remove_dir_all`] or the [`FileSystem`] equivalent) or moving
/// it to its final location.
///
/// # Arguments
///
/// * `fs` – The [`FileSystem`] used for file I/O and path canonicalization.
/// * `archive_path` – Path to the `.rawdist` archive to extract.
///
/// # Returns
///
/// * `Ok(PathBuf)` – The path to the persistent temporary directory
///   containing the extracted and verified files.
/// * `Err(RawdistError)` – If the archive is too large, contains a path
///   traversal, has an unrecognised entry type, fails checksum
///   verification, or encounters an I/O error.
///
/// # Panics
///
/// This function does not panic. All errors are returned.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::package::extract_to_temp;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let extracted = extract_to_temp(&fs, Path::new("package.rawdist"))
///     .expect("Extraction failed");
/// // Use extracted files...
/// // Then clean up:
/// std::fs::remove_dir_all(&extracted).unwrap();
/// ```
pub fn extract_to_temp(fs: &dyn FileSystem, archive_path: &Path) -> Result<PathBuf, RawdistError> {
    // Obtain the compressed file size before reading it completely.
    // This is a cheap check that prevents loading a maliciously large
    // file into memory.
    let metadata = fs.metadata(archive_path)?;
    let size = metadata.len();
    if size > MAX_ARCHIVE_SIZE {
        return Err(RawdistError::ArchiveTooLarge {
            size,
            max: MAX_ARCHIVE_SIZE,
        });
    }

    // Read the entire compressed archive into memory. For a 500 MB
    // maximum, this is acceptable; for larger limits a streaming
    // approach would be preferred.
    let data = fs.read(archive_path)?;
    let cursor = Cursor::new(data);
    let dec = GzDecoder::new(cursor);
    let mut archive = Archive::new(dec);

    // Create a temporary directory that will be automatically deleted
    // when `temp_dir` goes out of scope, unless we call `.keep()`.
    let temp_dir = tempfile::tempdir()?;
    let dest = temp_dir.path().to_path_buf();

    // Canonicalize the temporary directory to obtain a stable base for
    // path traversal checks. This resolves any symlinks in the temp
    // path itself.
    let dest_canonical = fs.canonicalize(&dest)?;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_path_buf();

        // Reject any path containing `..` components. This is the
        // first line of defense against classic path traversal
        // attacks embedded in tarball headers.
        if entry_path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(RawdistError::PathTraversal(entry_path));
        }

        // Accept only regular files and directories. Symlinks and
        // hard links are explicitly disallowed to prevent linking
        // to sensitive locations on the host system. This avoids
        // the need to audit link targets.
        let entry_type = entry.header().entry_type();
        match entry_type {
            EntryType::Regular | EntryType::Directory => {}
            _ => {
                return Err(RawdistError::PathTraversal(entry_path));
            }
        }

        // Build the final extraction path by joining the temp
        // directory and the entry's relative path. Parent
        // directories are created on‑demand.
        let joined = dest.join(&entry_path);
        if let Some(parent) = joined.parent() {
            fs.create_dir_all(parent)?;
            // Canonicalize the parent after creation to detect
            // symlinks embedded deeper in the tree that might
            // escape the intended root.
            let parent_canon = fs.canonicalize(parent)?;
            // If the canonical parent does not reside under the
            // canonical destination root, the tarball tried to
            // redirect extraction outside the temp directory via
            // a symlink.
            if !parent_canon.starts_with(&dest_canonical) {
                return Err(RawdistError::PathTraversal(entry_path));
            }
        }

        // Unpack the entry. If `unpack_in` fails, the temporary
        // directory will be cleaned up automatically when `temp_dir`
        // is dropped.
        entry.unpack_in(&dest)?;
    }

    // Verify that the mandatory checksums file exists. Its absence
    // indicates a malformed package.
    let checksum_file = dest.join("checksums.sha256");
    if !fs.exists(&checksum_file) {
        return Err(RawdistError::MissingFile {
            path: PathBuf::from("checksums.sha256"),
        });
    }

    // Parse the expected checksums and compare against the actual
    // files on disk. This ensures the archive's content matches what
    // the packager claimed.
    let content = fs.read_to_string(&checksum_file)?;
    let expected = checksum::parse_checksums(&content)?;
    for (rel_path, expected_hash) in &expected {
        let actual_path = dest.join(rel_path);
        if !fs.exists(&actual_path) {
            return Err(RawdistError::MissingFile {
                path: rel_path.clone(),
            });
        }
        let actual_hash = checksum::hash_file(fs, &actual_path)?;
        if &actual_hash != expected_hash {
            return Err(RawdistError::ChecksumMismatch { path: actual_path });
        }
    }

    // Remove the checksums file from the extracted directory because
    // it is an artifact of the packaging process, not part of the
    // package's actual file tree that should be installed.
    fs.remove_file(&checksum_file)?;

    // Keep the temporary directory alive and return its path. The
    // caller assumes responsibility for cleanup.
    let persistent = temp_dir.keep();
    Ok(persistent)
}

/// Moves (renames) an extracted package directory from `src` to `target`,
/// ensuring the target does not already exist.
///
/// This is intended to be used after a successful [`extract_to_temp`]: the
/// verified temporary directory is relocated to its final installation
/// location. The rename is typically atomic on the same file system,
/// avoiding race conditions during installation.
///
/// # Arguments
///
/// * `fs` – The [`FileSystem`] used for directory creation and rename.
/// * `src` – The source directory (usually the path returned by
///   [`extract_to_temp`]).
/// * `target` – The desired final path. Its parent directories are created
///   if they are missing.
///
/// # Returns
///
/// * `Ok(())` – The directory was successfully moved.
/// * `Err(RawdistError::Config)` – If `target` already exists, preventing
///   accidental overwrites.
/// * `Err(RawdistError)` – If parent directory creation fails or the rename
///   encounters an I/O error.
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::package::{extract_to_temp, move_extracted};
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let extracted = extract_to_temp(&fs, Path::new("pkg.rawdist")).unwrap();
/// move_extracted(&fs, &extracted, Path::new("/opt/rawssg/my_package"))
///     .expect("Move failed");
/// ```
pub fn move_extracted(fs: &dyn FileSystem, src: &Path, target: &Path) -> Result<(), RawdistError> {
    // Prevent overwriting an existing installation. This is a safety
    // measure to avoid data loss; callers must explicitly remove the
    // target first.
    if fs.exists(target) {
        return Err(RawdistError::Config(format!(
            "Target already exists: {}",
            target.display()
        )));
    }
    // Ensure the target's parent directory exists so that the rename
    // (which acts as a move) will succeed.
    if let Some(parent) = target.parent() {
        fs.create_dir_all(parent)?;
    }
    // Rename is preferred over copy+delete because it is fast and
    // atomic on most file systems when both paths reside on the same
    // volume.
    fs.rename(src, target)?;
    Ok(())
}
