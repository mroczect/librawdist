use librawdist::RawdistConfig;
use std::path::Path;

fn valid_config() -> RawdistConfig {
    toml::from_str(
        r#"
[package]
name = "valid-name"
version = "1.2.3"

[files]
include = ["*.txt"]
"#,
    )
    .unwrap()
}

#[test]
fn test_validate_valid() {
    let config = valid_config();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_empty_name() {
    let mut config = valid_config();
    config.package.name = "".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_invalid_name_chars() {
    let mut config = valid_config();
    config.package.name = "bad name!".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_invalid_version() {
    let mut config = valid_config();
    config.package.version = "not semver".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_target_dir_absolute() {
    let mut config = valid_config();
    config.install.target_dir = "/etc/passwd".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_target_dir_with_dotdot() {
    let mut config = valid_config();
    config.install.target_dir = "../escape".into();
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_no_include_patterns() {
    let mut config = valid_config();
    config.files.include = vec![];
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_merge_config_unsafe() {
    let mut config = valid_config();
    config.install.merge_config = Some("/etc/hosts".into());
    assert!(config.validate().is_err());
}

#[test]
fn test_load_from_dir_missing() {
    let err = RawdistConfig::load_from_dir(Path::new("/tmp/nonexistent_dir_")).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::MissingFile { .. }));
}

#[test]
fn test_load_from_dir_invalid_toml() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("rawdist.conf"), b"not toml").unwrap();
    let err = RawdistConfig::load_from_dir(dir.path()).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::TomlParse { .. }));
}
