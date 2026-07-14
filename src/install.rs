use crate::error::LibrawdistError;
use crate::types::{InstalledPackage, LibrawdistConfig};
use crate::{manifest, package, verify};
use std::path::{Path, PathBuf};

/// Install a `.rawdist` package from `archive_path` into the project.
/// `manifest_path` specifies the location of the manifest file (usually `rawssg-packages.toml`).
/// If `target_override` is `Some`, it will be used instead of the package's `target_dir`.
pub fn install_package(
    archive_path: &Path,
    target_override: Option<&Path>,
    manifest_path: &Path,
) -> Result<(), LibrawdistError> {
    if !archive_path.exists() {
        return Err(LibrawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }
    if archive_path.extension().map_or(true, |ext| ext != "rawdist") {
        return Err(LibrawdistError::InvalidInput("Expected .rawdist file".into()));
    }

    // Extract to temp and verify
    let extracted = package::extract_to_temp(archive_path)?;
    let config = LibrawdistConfig::load_from_dir(&extracted)?; // see impl below

    let target_dir = if let Some(t) = target_override {
        t.to_path_buf()
    } else {
        let resolved = config
            .install
            .target_dir
            .replace("{{ package.name }}", &config.package.name);
        PathBuf::from(resolved)
    };

    if target_dir.exists() {
        return Err(LibrawdistError::Config(format!(
            "Target directory '{}' already exists. Remove it first or use an override.",
            target_dir.display()
        )));
    }

    package::move_extracted(&extracted, &target_dir)?;

    // Update manifest
    let mut manifest = manifest::load_manifest(manifest_path)?;
    manifest
        .packages
        .retain(|p| p.name != config.package.name);
    manifest.packages.push(InstalledPackage {
        name: config.package.name.clone(),
        version: config.package.version.clone(),
        install_path: target_dir.canonicalize()?,
        config_merged: config.install.merge_config.clone(),
    });
    manifest::save_manifest(manifest_path, &manifest)?;

    log::info!(
        "Package '{}' installed to {}",
        config.package.name,
        target_dir.display()
    );
    Ok(())
}

/// Remove an installed package by name.
/// `manifest_path` is the path to the project's manifest.
pub fn remove_package(package_name: &str, manifest_path: &Path) -> Result<(), LibrawdistError> {
    let mut manifest = manifest::load_manifest(manifest_path)?;
    let pos = manifest
        .packages
        .iter()
        .position(|p| p.name == package_name)
        .ok_or_else(|| LibrawdistError::NotInstalled(package_name.to_string()))?;
    let pkg = manifest.packages.remove(pos);

    // Delete the installed directory
    if pkg.install_path.exists() {
        std::fs::remove_dir_all(&pkg.install_path)?;
        log::info!("Removed directory: {}", pkg.install_path.display());
    } else {
        log::warn!(
            "Install path not found, removing manifest entry only: {}",
            pkg.install_path.display()
        );
    }

    manifest::save_manifest(manifest_path, &manifest)?;
    log::info!("Package '{}' removed.", package_name);
    Ok(())
}

// Helper: load config from an extracted directory
impl LibrawdistConfig {
    fn load_from_dir(dir: &Path) -> Result<Self, LibrawdistError> {
        let config_path = dir.join("Librawdist.conf");
        if !config_path.exists() {
            return Err(LibrawdistError::MissingFile { path: config_path });
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content).map_err(|e| LibrawdistError::TomlParse {
            path: config_path,
            source: e,
        })?;
        config.validate()?;
        Ok(config)
    }
}
