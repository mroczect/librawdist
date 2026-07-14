mod common;
use librawdist::fs::RealFs;
use librawdist::{
    RawdistConfig, RawdistError, create_package, install_package, manifest, remove_package,
};
use std::path::Path;
use tempfile::TempDir;

fn make_dummy_config(dir: &Path, name: &str, version: &str) {
    let config_content = format!(
        r#"
[package]
name = "{name}"
version = "{version}"
[rawssg]
type = "theme"
[files]
include = ["*.txt"]
[install]
target_dir = "themes/{name}"
"#
    );
    std::fs::write(dir.join("rawdist.conf"), config_content).unwrap();
    std::fs::write(dir.join("file.txt"), b"test data").unwrap();
}

#[test]
fn install_and_remove() {
    let fs = RealFs;
    let src_dir = TempDir::new().unwrap();
    make_dummy_config(src_dir.path(), "testpkg", "1.0.0");
    let archive_path = src_dir.path().join("pkg.rawdist");
    let config = RawdistConfig::load_from_dir(&fs, src_dir.path()).unwrap();
    create_package(&fs, src_dir.path(), &archive_path, &config).unwrap();

    let dest_dir = TempDir::new().unwrap();
    let manifest_path = dest_dir.path().join("rawssg-packages.toml");

    install_package(
        &fs,
        &archive_path,
        Some(&dest_dir.path().join("custom_target")),
        &manifest_path,
    )
    .unwrap();
    assert!(dest_dir.path().join("custom_target").exists());
    let manifest = manifest::load_manifest(&fs, &manifest_path).unwrap();
    assert_eq!(manifest.packages[0].name, "testpkg");

    remove_package(&fs, "testpkg", &manifest_path).unwrap();
    assert!(!dest_dir.path().join("custom_target").exists());
}

#[test]
fn install_archive_not_found() {
    let fs = RealFs;
    let err = install_package(
        &fs,
        Path::new("/nonexistent.rawdist"),
        None,
        Path::new("/tmp/manifest"),
    )
    .unwrap_err();
    assert!(matches!(err, RawdistError::InvalidInput(_)));
}

#[test]
fn install_wrong_extension() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file.txt");
    std::fs::write(&path, b"x").unwrap();
    let err = install_package(&fs, &path, None, Path::new("/tmp/manifest")).unwrap_err();
    assert!(matches!(err, RawdistError::InvalidInput(_)));
}

#[test]
fn install_target_exists() {
    let fs = RealFs;
    let src_dir = TempDir::new().unwrap();
    make_dummy_config(src_dir.path(), "pkg", "1.0.0");
    let archive = src_dir.path().join("pkg.rawdist");
    let config = RawdistConfig::load_from_dir(&fs, src_dir.path()).unwrap();
    create_package(&fs, src_dir.path(), &archive, &config).unwrap();

    let dest = TempDir::new().unwrap();
    let target = dest.path().join("existing");
    std::fs::create_dir(&target).unwrap();
    let manifest = dest.path().join("manifest.toml");
    let err = install_package(&fs, &archive, Some(&target), &manifest).unwrap_err();
    assert!(matches!(err, RawdistError::Config(_)));
}

#[test]
fn remove_not_installed() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let manifest = tmp.path().join("manifest.toml");
    let err = remove_package(&fs, "ghost", &manifest).unwrap_err();
    assert!(matches!(err, RawdistError::NotInstalled(_)));
}
