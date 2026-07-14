mod common;
use common::MockFs;
use librawdist::RawdistError;
use librawdist::manifest;
use librawdist::types::{InstalledPackage, Manifest};
use std::path::{Path, PathBuf};

#[test]
fn load_default() {
    let fs = MockFs::new();
    let manifest = manifest::load_manifest(&fs, Path::new("/no.toml")).unwrap();
    assert!(manifest.packages.is_empty());
}

#[test]
fn save_and_load() {
    let mut fs = MockFs::new();
    let manifest = Manifest {
        packages: vec![InstalledPackage {
            name: "test".into(),
            version: "0.1".into(),
            install_path: PathBuf::from("/tmp/test"),
            config_merged: None,
        }],
    };
    manifest::save_manifest(&fs, Path::new("/manifest.toml"), &manifest).unwrap();
    // Because mock write doesn't persist, we simulate by setting content for load
    let toml_str = toml::to_string(&manifest).unwrap();
    fs.add_file(Path::new("/manifest.toml"), toml_str.as_bytes());
    let loaded = manifest::load_manifest(&fs, Path::new("/manifest.toml")).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].name, "test");
}

#[test]
fn load_invalid_toml() {
    let mut fs = MockFs::new();
    fs.add_file(Path::new("/bad.toml"), b"invalid = toml::");
    let err = manifest::load_manifest(&fs, Path::new("/bad.toml")).unwrap_err();
    assert!(matches!(err, RawdistError::TomlParse { .. }));
}

#[test]
fn save_error() {
    let mut fs = MockFs::new();
    fs.write_error = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let manifest = Manifest::default();
    let err = manifest::save_manifest(&fs, Path::new("/any"), &manifest).unwrap_err();
    assert!(matches!(err, RawdistError::Config(_) | RawdistError::Io(_)));
}
