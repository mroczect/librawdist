use crate::error::RawdistError;
use crate::types::{InstalledPackage, RawdistConfig};
use crate::{fs, manifest, package};
use std::path::{Path, PathBuf};

pub fn install_package(
    archive_path: &Path,
    target_override: Option<&Path>,
    manifest_path: &Path,
) -> Result<(), RawdistError> {
    if !archive_path.exists() {
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

    let fs = fs::RealFs;
    let extracted = package::extract_to_temp(&fs, archive_path)?;
    let config = RawdistConfig::load_from_dir(&extracted)?;

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
        return Err(RawdistError::Config(format!(
            "Target directory '{}' already exists. Remove it first or use an override.",
            target_dir.display()
        )));
    }

    package::move_extracted(&extracted, &target_dir)?;

    let mut manifest = manifest::load_manifest(manifest_path)?;
    manifest.packages.retain(|p| p.name != config.package.name);
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

pub fn remove_package(package_name: &str, manifest_path: &Path) -> Result<(), RawdistError> {
    let mut manifest = manifest::load_manifest(manifest_path)?;
    let pos = manifest
        .packages
        .iter()
        .position(|p| p.name == package_name)
        .ok_or_else(|| RawdistError::NotInstalled(package_name.to_string()))?;
    let pkg = manifest.packages.remove(pos);

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
