use librawdist::manifest;
use librawdist::types::{InstalledPackage, Manifest};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_load_default() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("no.toml");
    let manifest = manifest::load_manifest(&path).unwrap();
    assert!(manifest.packages.is_empty());
}

#[test]
fn test_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("manifest.toml");
    let mut manifest = Manifest::default();
    manifest.packages.push(InstalledPackage {
        name: "test".into(),
        version: "0.1".into(),
        install_path: PathBuf::from("/tmp/test"),
        config_merged: None,
    });
    manifest::save_manifest(&path, &manifest).unwrap();
    let loaded = manifest::load_manifest(&path).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].name, "test");
}