mod common;
use common::MockFs;
use librawdist::RawdistError;
use librawdist::types::{
    FilePatterns, InstallConfig, InstalledPackage, Manifest, PackageMeta, RawdistConfig, RawssgReqs,
};
use std::path::Path;

fn valid_config() -> RawdistConfig {
    RawdistConfig::new(
        PackageMeta {
            name: "valid-name".into(),
            version: "1.2.3".into(),
            description: None,
            authors: None,
            license: None,
            repository: None,
            homepage: None,
            documentation: None,
            keywords: None,
            categories: None,
            release_date: None,
        },
        RawssgReqs {
            min_version: None,
            max_version: None,
            r#type: "theme".into(),
        },
        FilePatterns {
            include: vec!["*.txt".to_string()],
            exclude: vec![],
        },
        InstallConfig {
            target_dir: "themes/valid".into(),
            merge_config: None,
        },
    )
}

#[test]
fn validate_valid() {
    assert!(valid_config().validate().is_ok());
}

#[test]
fn validate_empty_name() {
    let mut c = valid_config();
    c.package.name = "".into();
    assert!(c.validate().is_err());
}

#[test]
fn validate_invalid_name_chars() {
    let mut c = valid_config();
    c.package.name = "bad name!".into();
    assert!(c.validate().is_err());
}

#[test]
fn validate_invalid_version() {
    let mut c = valid_config();
    c.package.version = "not semver".into();
    assert!(c.validate().is_err());
}

#[test]
fn validate_target_dir_absolute() {
    let mut c = valid_config();
    c.install.target_dir = "/etc/passwd".into();
    assert!(c.validate().is_err());
}

#[test]
fn validate_target_dir_with_dotdot() {
    let mut c = valid_config();
    c.install.target_dir = "../escape".into();
    assert!(c.validate().is_err());
}

#[test]
fn validate_no_include_patterns() {
    let mut c = valid_config();
    c.files.include = vec![];
    assert!(c.validate().is_err());
}

#[test]
fn validate_merge_config_unsafe() {
    let mut c = valid_config();
    c.install.merge_config = Some("/etc/hosts".into());
    assert!(c.validate().is_err());
}

// ---------- load_from_dir ----------

#[test]
fn load_from_dir_missing() {
    let fs = MockFs::new();
    let err = RawdistConfig::load_from_dir(&fs, Path::new("/nonexistent")).unwrap_err();
    assert!(matches!(err, RawdistError::MissingFile { .. }));
}

#[test]
fn load_from_dir_invalid_toml() {
    let mut fs = MockFs::new();
    fs.add_file(Path::new("/test/rawdist.conf"), b"not toml");
    let err = RawdistConfig::load_from_dir(&fs, Path::new("/test")).unwrap_err();
    assert!(matches!(err, RawdistError::TomlParse { .. }));
}

#[test]
fn load_from_dir_validation_error() {
    let mut fs = MockFs::new();
    let config_toml = r#"
[package]
name = ""
version = "0.1.0"

[rawssg]
type = "theme"

[files]
include = ["*.txt"]

[install]
target_dir = "ok"
"#;
    fs.add_file(Path::new("/test/rawdist.conf"), config_toml.as_bytes());
    let err = RawdistConfig::load_from_dir(&fs, Path::new("/test")).unwrap_err();
    assert!(matches!(err, RawdistError::Validation(_)));
}

// ---------- resolve_target_dir ----------

#[test]
fn resolve_target_dir_simple() {
    let config = valid_config();
    let resolved = config.resolve_target_dir();
    // default target_dir = "themes/valid", no template
    assert_eq!(resolved, "themes/valid");
}

#[test]
fn resolve_target_dir_with_package_name() {
    let mut config = valid_config();
    config.install.target_dir = "themes/{{ package.name }}".into();
    assert_eq!(config.resolve_target_dir(), "themes/valid-name");
}

#[test]
fn resolve_target_dir_with_package_version() {
    let mut config = valid_config();
    config.install.target_dir = "themes/{{ package.version }}".into();
    assert_eq!(config.resolve_target_dir(), "themes/1.2.3");
}

#[test]
fn resolve_target_dir_mixed() {
    let mut config = valid_config();
    config.install.target_dir = "{{ package.name }}-{{ package.version }}".into();
    assert_eq!(config.resolve_target_dir(), "valid-name-1.2.3");
}

// ---------- Serialization ----------

#[test]
fn manifest_serialization_roundtrip() {
    let mut manifest = Manifest::default();
    manifest.packages.push(InstalledPackage {
        name: "pkg".into(),
        version: "1.0".into(),
        install_path: Path::new("/tmp/pkg").to_path_buf(),
        config_merged: None,
    });
    let toml_str = toml::to_string(&manifest).unwrap();
    let loaded: Manifest = toml::from_str(&toml_str).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].name, "pkg");
}

#[test]
fn validate_edition_invalid() {
    let mut config = valid_config();
    config.edition = "99".into();
    assert!(config.validate().is_err());
}
