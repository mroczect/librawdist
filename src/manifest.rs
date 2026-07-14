use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::Manifest;
use std::path::Path;

/// Loads the package manifest from a TOML file, returning a default empty
/// manifest if the file does not exist.
///
/// This design allows the manifest file to be optional: new projects do not
/// need to create an empty file before installing packages. The first
/// successful package installation will create or overwrite the manifest.
///
/// # Arguments
///
/// * `fs` – The [`FileSystem`] implementation used to read the file.
/// * `path` – The path to the manifest file (typically
///   `rawssg-packages.toml`).
///
/// # Returns
///
/// * `Ok(Manifest)` – The parsed manifest, or an empty default if the file
///   is absent.
/// * `Err(RawdistError)` – Wraps I/O errors or TOML parsing failures in
///   [`RawdistError::TomlParse`] with the file path and the underlying
///   `toml::de::Error`.
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::manifest::load_manifest;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let manifest = load_manifest(&fs, Path::new("rawssg-packages.toml"))
///     .expect("Failed to load manifest");
/// assert!(manifest.packages.is_empty()); // No file exists yet.
/// ```
pub fn load_manifest(fs: &dyn FileSystem, path: &Path) -> Result<Manifest, RawdistError> {
    // If the manifest file doesn't exist, return a default empty manifest.
    // This avoids forcing users to run an explicit `init` step and keeps
    // the initial setup as simple as possible.
    if !fs.exists(path) {
        return Ok(Manifest::default());
    }

    // Read the entire file into a string. Using `read_to_string` ensures
    // that the TOML parser receives a contiguous UTF-8 buffer, which is
    // the most common case for configuration files.
    let content = fs.read_to_string(path)?;

    // Deserialize the TOML. The error from `toml::from_str` is mapped into
    // a `RawdistError::TomlParse` to provide consistent error reporting
    // across all TOML-handling code paths, including the file path for
    // diagnostic context.
    let manifest: Manifest = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(manifest)
}

/// Serializes a [`Manifest`] to pretty-printed TOML and writes it to disk.
///
/// The file is created (or overwritten) and all necessary parent
/// directories are created by the underlying [`FileSystem::write`]
/// implementation.
///
/// # Arguments
///
/// * `fs` – The [`FileSystem`] used to write the file.
/// * `path` – The destination path for the manifest.
/// * `manifest` – The manifest data structure to persist.
///
/// # Returns
///
/// * `Ok(())` – The manifest was successfully written.
/// * `Err(RawdistError)` – If TOML serialization fails (mapped to
///   [`RawdistError::Config`]) or the file cannot be written.
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::manifest::save_manifest;
/// use librawdist::fs::RealFs;
/// use librawdist::types::Manifest;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let manifest = Manifest::default();
/// save_manifest(&fs, Path::new("rawssg-packages.toml"), &manifest)
///     .expect("Failed to save manifest");
/// ```
pub fn save_manifest(
    fs: &dyn FileSystem,
    path: &Path,
    manifest: &Manifest,
) -> Result<(), RawdistError> {
    // Pretty-print the manifest for human readability. This is beneficial
    // because the manifest is a user-facing file that may be inspected or
    // edited manually. The `Config` variant is used for serialization
    // errors because they indicate a data model issue, not a TOML syntax
    // problem at the file level.
    let content =
        toml::to_string_pretty(manifest).map_err(|e| RawdistError::Config(e.to_string()))?;

    // Write the serialized content to the file. The `write` method of
    // `FileSystem` handles parent directory creation, so callers do not
    // need to ensure that the target directory exists.
    fs.write(path, content.as_bytes())?;
    Ok(())
}
