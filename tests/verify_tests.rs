mod common;
use common::MockFs;
use librawdist::fs::RealFs;
use librawdist::{RawdistConfig, RawdistError, create_package, verify_package};
use std::path::Path;
use tempfile::TempDir;

#[test]
fn verify_archive_not_found() {
    let fs = RealFs;
    let err = verify_package(&fs, Path::new("/nonexistent.rawdist"), false).unwrap_err();
    assert!(matches!(err, RawdistError::InvalidInput(_)));
}

#[test]
fn verify_valid_archive_and_keep_temp() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(
        src.join("rawdist.conf"),
        br#"
[package]
name = "p"
version = "0.1.0"
[rawssg]
type = "theme"
[files]
include = ["*.txt"]
[install]
target_dir = "themes/p"
"#,
    )
    .unwrap();
    std::fs::write(src.join("a.txt"), b"data").unwrap();
    let config = RawdistConfig::load_from_dir(&fs, &src).unwrap();
    let archive = tmp.path().join("p.rawdist");
    create_package(&fs, &src, &archive, &config).unwrap();

    let kept = verify_package(&fs, &archive, true).unwrap();
    assert!(kept.is_some());
    let path = kept.unwrap();
    assert!(path.exists());
    std::fs::remove_dir_all(&path).unwrap();

    let none_kept = verify_package(&fs, &archive, false).unwrap();
    assert!(none_kept.is_none());
}

#[test]
fn verify_error_during_extraction() {
    let mut mock = MockFs::new();
    mock.metadata_error = Some(std::io::Error::new(std::io::ErrorKind::Other, "fail"));
    mock.add_file(Path::new("/bad.rawdist"), b"dummy");
    let err = verify_package(&mock, Path::new("/bad.rawdist"), false).unwrap_err();
    assert!(matches!(
        err,
        RawdistError::Io(_) | RawdistError::ArchiveTooLarge { .. }
    ));
}
