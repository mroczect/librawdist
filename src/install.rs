use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::{InstalledPackage, RawdistConfig};
use crate::{manifest, package};
use std::path::{Path, PathBuf};

pub fn install_package(
    fs: &dyn FileSystem,
    archive_path: &Path,
    target_override: Option<&Path>,
    manifest_path: &Path,
) -> Result<(), RawdistError> {
    if !fs.exists(archive_path) {
        return Err(RawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }
    if archive_path
        .extension()
        .map_or(true, |ext| ext != "rawdist")
    {
        return Err(RawdistError::InvalidInput("Expected .rawdist file".into()));
    }

    let extracted = package::extract_to_temp(fs, archive_path)?;
    let config = RawdistConfig::load_from_dir(fs, &extracted)?;

    let target_dir = if let Some(t) = target_override {
        t.to_path_buf()
    } else {
        PathBuf::from(config.resolve_target_dir())
    };

    if fs.exists(&target_dir) {
        return Err(RawdistError::Config(format!(
            "Target directory '{}' already exists. Remove it first or use an override.",
            target_dir.display()
        )));
    }

    package::move_extracted(fs, &extracted, &target_dir)?;

    let mut manifest = manifest::load_manifest(fs, manifest_path)?;
    manifest.packages.retain(|p| p.name != config.package.name);
    manifest.packages.push(InstalledPackage {
        name: config.package.name.clone(),
        version: config.package.version.clone(),
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

pub fn remove_package(
    fs: &dyn FileSystem,
    package_name: &str,
    manifest_path: &Path,
) -> Result<(), RawdistError> {
    let mut manifest = manifest::load_manifest(fs, manifest_path)?;
    let pos = manifest
        .packages
        .iter()
        .position(|p| p.name == package_name)
        .ok_or_else(|| RawdistError::NotInstalled(package_name.to_string()))?;
    let pkg = manifest.packages.remove(pos);

    if fs.exists(&pkg.install_path) {
        fs.remove_dir_all(&pkg.install_path)?;
        log::info!("Removed directory: {}", pkg.install_path.display());
    } else {
        log::warn!(
            "Install path not found, removing manifest entry only: {}",
            pkg.install_path.display()
        );
    }

    manifest::save_manifest(fs, manifest_path, &manifest)?;
    log::info!("Package '{}' removed.", package_name);
    Ok(())
}
