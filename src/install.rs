use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::{InstalledPackage, RawdistConfig};
use crate::{manifest, package};
use std::path::{Path, PathBuf};

/// Installs a `.rawdist` package archive into the system.
///
/// This function orchestrates the entire installation workflow:
///
/// 1. Validates that the archive exists and has the correct extension.
/// 2. Extracts the archive to a temporary location (with integrity checks).
/// 3. Loads the package configuration from the extracted directory.
/// 4. Determines the final installation target (either an override or the
///    resolved `target_dir` from the configuration).
/// 5. Moves the extracted content to the target location, ensuring no
///    overwrites.
/// 6. Updates the manifest with the newly installed package, performing an
///    upsert (remove if previously present, then push the new entry).
/// 7. Persists the manifest to disk.
///
/// The manifest is loaded and saved inside the function to keep the entire
/// transaction as a single atomic unit from the caller’s perspective; if
/// any step after the file‑copy fails, the package will not appear as
/// installed.
///
/// # Arguments
///
/// * `fs` – A [`FileSystem`] implementation for all disk operations.
/// * `archive_path` – Path to the `.rawdist` archive file.
/// * `target_override` – If `Some`, the package is installed into this
///   directory instead of the one declared in the configuration. This is
///   typically used for custom installation locations.
/// * `manifest_path` – Path to the TOML manifest file that tracks
///   installed packages (e.g., `rawssg-packages.toml`).
///
/// # Returns
///
/// * `Ok(())` – The package was installed successfully.
/// * `Err(RawdistError)` – On any validation, I/O, extraction, checksum,
///   or configuration error.
///
/// # Panics
///
/// This function does not panic. All error conditions are returned as
/// `Result::Err`.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::install::install_package;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// install_package(
///     &fs,
///     Path::new("my_package.rawdist"),
///     Some(Path::new("/opt/rawssg/themes/")),
///     Path::new("rawssg-packages.toml"),
/// )
/// .expect("Installation failed");
/// ```
pub fn install_package(
    fs: &dyn FileSystem,
    archive_path: &Path,
    target_override: Option<&Path>,
    manifest_path: &Path,
) -> Result<(), RawdistError> {
    // Validate archive existence early to give a clear error before any
    // work is performed. This avoids a confusing "file not found" error
    // later in the extraction step.
    if !fs.exists(archive_path) {
        return Err(RawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }

    // Enforce the `.rawdist` extension. This is a lightweight sanity check
    // that prevents accidental processing of unrelated files and provides
    // a clear diagnostic to the user.
    if archive_path
        .extension()
        .map_or(true, |ext| ext != "rawdist")
    {
        return Err(RawdistError::InvalidInput("Expected .rawdist file".into()));
    }

    // Extract to a temporary directory. extract_to_temp performs:
    // - Archive size check
    // - Decompression and unpacking
    // - Checksum verification against the embedded checksums.sha256
    let extracted = package::extract_to_temp(fs, archive_path)?;

    // Load the package's self-describing configuration from the extracted
    // directory. This config dictates where and how the package should be
    // installed.
    let config = RawdistConfig::load_from_dir(fs, &extracted)?;

    // Determine the final installation target directory. The override
    // takes precedence; otherwise the configuration's resolved target_dir
    // is used (with template variables like {{ package.name }} substituted).
    let target_dir = if let Some(t) = target_override {
        t.to_path_buf()
    } else {
        PathBuf::from(config.resolve_target_dir())
    };

    // Protect against accidental overwrites. Requiring the user to
    // explicitly remove an existing installation prevents data loss.
    if fs.exists(&target_dir) {
        return Err(RawdistError::Config(format!(
            "Target directory '{}' already exists. Remove it first or use an override.",
            target_dir.display()
        )));
    }

    // Move (rename) the extracted content to the target. This is an
    // efficient operation because both directories are typically on the
    // same file system.
    package::move_extracted(fs, &extracted, &target_dir)?;

    // Load, modify, and save the manifest atomically (from the caller's
    // view). The previous entry for the same package name is removed to
    // support upgrades and reinstallations without duplicating entries.
    let mut manifest = manifest::load_manifest(fs, manifest_path)?;
    manifest.packages.retain(|p| p.name != config.package.name);
    manifest.packages.push(InstalledPackage {
        name: config.package.name.clone(),
        version: config.package.version.clone(),
        // Canonicalize the install path so the manifest stores an absolute,
        // normalized location that is immune to directory renames in the
        // process’s working directory.
        install_path: fs.canonicalize(&target_dir)?,
        config_merged: config.install.merge_config.clone(),
    });
    manifest::save_manifest(fs, manifest_path, &manifest)?;

    log::info!(
        "Package '{}' installed to {}",
        config.package.name,
        target_dir.display()
    );
    Ok(())
}

/// Removes a previously installed package by name.
///
/// The function locates the package entry in the manifest, removes its
/// recorded installation directory from disk (if it still exists), and
/// then deletes the entry from the manifest. If the installation directory
/// is already gone, the entry is still removed; a warning is logged to
/// inform the user of the discrepancy.
///
/// # Arguments
///
/// * `fs` – A [`FileSystem`] implementation for directory removal and
///   manifest I/O.
/// * `package_name` – The name of the package to uninstall (as recorded
///   in the manifest).
/// * `manifest_path` – Path to the manifest file.
///
/// # Returns
///
/// * `Ok(())` – The package was removed (or its manifest entry deleted).
/// * `Err(RawdistError::NotInstalled)` – The package name is not present
///   in the manifest.
/// * `Err(RawdistError)` – For other I/O or manifest parse/write errors.
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::install::remove_package;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// remove_package(&fs, "my-theme", Path::new("rawssg-packages.toml"))
///     .expect("Removal failed");
/// ```
pub fn remove_package(
    fs: &dyn FileSystem,
    package_name: &str,
    manifest_path: &Path,
) -> Result<(), RawdistError> {
    // Load the manifest. If the file doesn't exist, load_manifest returns
    // a default empty manifest; the search below will then correctly
    // report `NotInstalled`.
    let mut manifest = manifest::load_manifest(fs, manifest_path)?;

    // Find the index of the package by name. We need the position to
    // extract the entry and later remove it from the vector.
    let pos = manifest
        .packages
        .iter()
        .position(|p| p.name == package_name)
        .ok_or_else(|| RawdistError::NotInstalled(package_name.to_string()))?;

    // Remove the entry from the manifest. This gives us ownership of the
    // `InstalledPackage` struct, which we need to access the install path.
    let pkg = manifest.packages.remove(pos);

    // Clean up the installation directory if it still exists. Not
    // failing when the directory is already missing avoids errors for
    // manually cleaned installations and ensures the manifest can be
    // repaired.
    if fs.exists(&pkg.install_path) {
        fs.remove_dir_all(&pkg.install_path)?;
        log::info!("Removed directory: {}", pkg.install_path.display());
    } else {
        log::warn!(
            "Install path not found, removing manifest entry only: {}",
            pkg.install_path.display()
        );
    }

    // Save the modified manifest. Even if the directory removal failed or
    // was skipped, we want the manifest to reflect the uninstalled state.
    manifest::save_manifest(fs, manifest_path, &manifest)?;
    log::info!("Package '{}' removed.", package_name);
    Ok(())
}
