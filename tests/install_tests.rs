use librawdist::{create_package, install_package, remove_package, RawdistConfig, RawdistError};
use std::path::Path;
use tempfile::TempDir;

fn make_dummy_config(dir: &Path, name: &str, version: &str) {
    let config_content = format!(
        r#"
[package]
name = "{}"
version = "{}"

[files]
include = ["*.txt"]

[install]
target_dir = "dummy_target/{{{{ package.name }}}}"
"#,
        name, version
    );
    std::fs::write(dir.join("rawdist.conf"), config_content).unwrap();
    std::fs::write(dir.join("file.txt"), b"test data").unwrap();
}

#[test]
fn test_install_and_remove() {
    let src_dir = TempDir::new().unwrap();
    make_dummy_config(src_dir.path(), "testpkg", "1.0.0");
    let archive_path = src_dir.path().join("pkg.rawdist");
    let config = RawdistConfig::load_from_dir(src_dir.path()).unwrap();
    let fs = librawdist::fs::RealFs;
    create_package(&fs, src_dir.path(), &archive_path, &config).unwrap();

    let dest_dir = TempDir::new().unwrap();
    let manifest_path = dest_dir.path().join("rawssg-packages.toml");

    // Install
    install_package(
        &archive_path,
        Some(&dest_dir.path().join("custom_target")),
        &manifest_path,
    )
    .unwrap();

    assert!(dest_dir.path().join("custom_target").exists());
    let manifest = librawdist::manifest::load_manifest(&manifest_path).unwrap();
    assert_eq!(manifest.packages.len(), 1);
    assert_eq!(manifest.packages[0].name, "testpkg");

    // Remove
    remove_package("testpkg", &manifest_path).unwrap();
    assert!(!dest_dir.path().join("custom_target").exists());
    let manifest_after = librawdist::manifest::load_manifest(&manifest_path).unwrap();
    assert!(manifest_after.packages.is_empty());
}

#[test]
fn test_install_archive_not_found() {
    let err = install_package(
        Path::new("/nonexistent.rawdist"),
        None,
        Path::new("/tmp/manifest.toml"),
    )
    .unwrap_err();
    assert!(matches!(err, RawdistError::InvalidInput(_)));
}

#[test]
fn test_install_wrong_extension() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file.txt");
    std::fs::write(&path, b"x").unwrap();
    let err = install_package(&path, None, Path::new("/tmp/manifest.toml")).unwrap_err();
    assert!(matches!(err, RawdistError::InvalidInput(_)));
}

#[test]
fn test_install_target_exists() {
    let src_dir = TempDir::new().unwrap();
    make_dummy_config(src_dir.path(), "pkg", "1.0.0");
    let archive = src_dir.path().join("pkg.rawdist");
    let config = RawdistConfig::load_from_dir(src_dir.path()).unwrap();
    create_package(&librawdist::fs::RealFs, src_dir.path(), &archive, &config).unwrap();

    let dest = TempDir::new().unwrap();
    let target = dest.path().join("existing");
    std::fs::create_dir(&target).unwrap();
    let manifest = dest.path().join("manifest.toml");
    let err = install_package(&archive, Some(&target), &manifest).unwrap_err();
    assert!(matches!(err, RawdistError::Config(_)));
}

#[test]
fn test_remove_not_installed() {
    let tmp = TempDir::new().unwrap();
    let manifest = tmp.path().join("manifest.toml");
    let err = remove_package("ghost", &manifest).unwrap_err();
    assert!(matches!(err, RawdistError::NotInstalled(_)));
}
